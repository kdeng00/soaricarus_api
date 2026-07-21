pub mod request {
    pub mod create_metadata {
        #[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Request {
            pub title: String,
            pub artist: String,
            pub album_artist: String,
            pub album: String,
            pub genre: String,
            pub date: String,
            pub track: i32,
            pub disc: i32,
            pub track_count: i32,
            pub disc_count: i32,
            pub duration: i32,
            pub audio_type: String,
            pub user_id: uuid::Uuid,
            pub song_queue_id: uuid::Uuid,
        }

        impl Request {
            pub fn is_valid(&self) -> bool {
                !self.title.is_empty()
                    || !self.artist.is_empty()
                    || !self.album_artist.is_empty()
                    || !self.album.is_empty()
                    || !self.genre.is_empty()
                    || !self.date.is_empty()
                    || self.track > 0
                    || self.disc > 0
                    || self.track_count > 0
                    || self.disc_count > 0
                    || self.duration > 0
                    || !self.audio_type.is_empty()
                    || !self.user_id.is_nil()
                    || !self.song_queue_id.is_nil()
            }

            pub fn to_song(&self) -> simodels::song::Song {
                simodels::song::Song {
                    id: uuid::Uuid::nil(),
                    title: self.title.clone(),
                    artist: self.artist.clone(),
                    album_artist: self.album_artist.clone(),
                    album: self.album.clone(),
                    genre: self.genre.clone(),
                    year: self.date[..3].parse().unwrap(),
                    track: self.track,
                    disc: self.disc,
                    track_count: self.track_count,
                    disc_count: self.disc_count,
                    duration: self.duration,
                    audio_type: self.audio_type.clone(),
                    user_id: self.user_id,
                    ..Default::default()
                }
            }
        }
    }

    pub mod get_songs {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Params {
            pub id: Option<uuid::Uuid>,
        }
    }
}

pub mod response {
    pub mod create_metadata {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<simodels::song::Song>,
        }
    }

    pub mod get_songs {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<simodels::song::Song>,
        }
    }

    pub mod delete_song {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct SongAndCoverArt {
            pub song: simodels::song::Song,
            pub coverart: simodels::coverart::CoverArt,
        }

        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<SongAndCoverArt>,
        }
    }
}

// TODO: Might make a distinction between year and date in a song's tag at some point

/// Module for song related endpoints
pub mod endpoint {
    use axum::{Json, response::IntoResponse};

    use crate::repo;
    use crate::repo::queue as repo_queue;

    // TODO: Change the name of this endpoint. Including the function name and path
    /// Endpoint to create song
    #[utoipa::path(
        post,
        path = super::super::queue::endpoints::QUEUEMETADATA,
        request_body(
            content = super::request::create_metadata::Request,
            description = "Data needed to create the song and save it to the filesystem",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Song created", body = super::response::create_metadata::Response),
            (status = 400, description = "Error", body = super::response::create_metadata::Response),
            (status = 505, description = "Error creating song", body = super::response::create_metadata::Response)
        )
    )]
    pub async fn create_metadata(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::Json(payload): axum::Json<super::request::create_metadata::Request>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::create_metadata::Response>,
    ) {
        let mut response = super::response::create_metadata::Response::default();

        if payload.is_valid() {
            let mut song = payload.to_song();
            song.filename =
                simodels::song::generate_filename(simodels::types::MusicType::FlacExtension, true)
                    .unwrap();
            song.directory = sienvy::environment::get_root_directory().value;

            match repo_queue::song::get_data(&pool, &payload.song_queue_id).await {
                Ok(data) => {
                    song.data = data;
                    let dir = std::path::Path::new(&song.directory);
                    if !dir.exists() {
                        println!("Creating directory");
                        match std::fs::create_dir_all(dir) {
                            Ok(_) => {
                                println!("Successfully created directory");
                            }
                            Err(err) => {
                                eprintln!("Error: Unable to create the directory {err:?}");
                            }
                        }
                    }

                    match song.save_to_filesystem() {
                        Ok(_) => match repo::song::insert(&pool, &song).await {
                            Ok((date_created, id)) => {
                                song.id = id;
                                song.date_created = Some(date_created);
                                response.message = String::from("Successful");
                                response.data.push(song);

                                (axum::http::StatusCode::OK, axum::Json(response))
                            }
                            Err(err) => {
                                response.message = format!("{:?} song {:?}", err.to_string(), song);
                                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                            }
                        },
                        Err(err) => {
                            response.message = err.to_string();
                            (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                axum::Json(response),
                            )
                        }
                    }
                }
                Err(err) => {
                    response.message = err.to_string();
                    (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                }
            }
        } else {
            response.message = String::from("Request body is not valid");
            (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
        }
    }

    // Endpoint to get songs
    #[utoipa::path(
        get,
        path = super::super::endpoints::GETSONGS,
        params(
            ("id" = uuid::Uuid, Path, description = "Id of song")
            ),
        responses(
            (status = 200, description = "Songs found", body = super::response::get_songs::Response),
            (status = 400, description = "Error getting songs", body = super::response::get_songs::Response)
        )
    )]
    pub async fn get_songs(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Query(params): axum::extract::Query<super::request::get_songs::Params>,
    ) -> (
        axum::http::StatusCode,
        Json<super::response::get_songs::Response>,
    ) {
        let mut response = super::response::get_songs::Response::default();

        match params.id {
            Some(id) => match repo::song::get_song(&pool, &id).await {
                Ok(song) => {
                    response.message = String::from(super::super::response::SUCCESSFUL);
                    response.data.push(song);
                    (axum::http::StatusCode::OK, axum::Json(response))
                }
                Err(err) => {
                    response.message = err.to_string();
                    (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                }
            },
            None => {
                response.message = String::from("Invalid parameters");
                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
            }
        }
    }

    /// Endpoint to get all songs
    #[utoipa::path(
        get,
        path = super::super::endpoints::GETALLSONGS,
        responses(
            (status = 200, description = "Getting all songs", body = super::response::get_songs::Response),
            (status = 404, description = "Song not found", body = super::response::get_songs::Response)
        )
    )]
    pub async fn get_all_songs(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::get_songs::Response>,
    ) {
        let mut response = super::response::get_songs::Response::default();

        match repo::song::get_all_songs(&pool).await {
            Ok(songs) => {
                if songs.is_empty() {
                    response.message = String::from("No songs available");
                    (axum::http::StatusCode::NOT_FOUND, axum::Json(response))
                } else {
                    response.message = String::from(super::super::response::SUCCESSFUL);
                    response.data = songs;
                    (axum::http::StatusCode::OK, axum::Json(response))
                }
            }
            Err(err) => {
                response.message = err.to_string();
                (axum::http::StatusCode::NOT_FOUND, axum::Json(response))
            }
        }
    }

    /// Ednpoint to stream song
    #[utoipa::path(
        get,
        path = super::super::endpoints::STREAMSONG,
        params(("id" = uuid::Uuid, Path, description = "Song Id")),
        responses(
            (status = 200, description = "Stream song", body = Vec<u8>),
            (status = 500, description = "Error streaming song", body = (u64, String))
        )
    )]
    pub async fn stream_song(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    ) -> impl IntoResponse {
        match repo::song::get_song(&pool, &id).await {
            Ok(song) => {
                let song_path = song.song_path().unwrap();
                let path = std::path::Path::new(&song_path);

                if !path.starts_with(&song.directory) || !path.exists() {
                    return Err((axum::http::StatusCode::NOT_FOUND, "File not found"));
                }

                let file = match tokio::fs::File::open(&path).await {
                    Ok(file) => file,
                    Err(_) => return Err((axum::http::StatusCode::NOT_FOUND, "File not found")),
                };

                let file_size = match file.metadata().await {
                    Ok(meta) => meta.len(),
                    Err(_) => {
                        return Err((
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            "Could not read file",
                        ));
                    }
                };

                let mime = mime_guess::from_path(path).first_or_octet_stream();
                let stream = tokio_util::io::ReaderStream::new(file);

                let rep = axum::response::Response::builder()
                    .header("content-type", mime.to_string())
                    .header("accept-ranges", "bytes")
                    .header("content-length", file_size.to_string())
                    .body(axum::body::Body::from_stream(stream))
                    .unwrap();

                Ok(rep)
            }
            Err(_err) => Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Could not find file",
            )),
        }
    }

    /// Endpoint to download song
    #[utoipa::path(
        get,
        path = super::super::endpoints::DOWNLOADSONG,
        params(("id" = uuid::Uuid, Path, description = "Song Id")),
        responses(
            (status = 200, description = "Download song", body = (u64, Vec<u8>)),
            (status = 404, description = "Song not found", body = (u64, Vec<u8>)),
            (status = 400, description = "Error downloading song", body = (u64, Vec<u8>))
        )
    )]
    pub async fn download_song(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    ) -> (axum::http::StatusCode, axum::response::Response) {
        match repo::song::get_song(&pool, &id).await {
            Ok(song) => match simodels::song::io::to_data(&song) {
                Ok(data) => {
                    let bytes = axum::body::Bytes::from(data);
                    let mut response = bytes.into_response();
                    let headers = response.headers_mut();
                    headers.insert(
                        axum::http::header::CONTENT_TYPE,
                        "audio/flac".parse().unwrap(),
                    );
                    headers.insert(
                        axum::http::header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{id}.flac\"")
                            .parse()
                            .unwrap(),
                    );

                    (axum::http::StatusCode::OK, response)
                }
                Err(_err) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::response::Response::default(),
                ),
            },
            Err(_err) => (
                axum::http::StatusCode::NOT_FOUND,
                axum::response::Response::default(),
            ),
        }
    }

    /// Endpoint to delete the song
    #[utoipa::path(
        delete,
        path = super::super::endpoints::DELETESONG,
        params(("id" = uuid::Uuid, Path, description = "Song Id")),
        responses(
            (status = 200, description = "Song deleted", body = super::response::delete_song::Response),
            (status = 404, description = "Song not found", body = super::response::delete_song::Response),
            (status = 500, description = "Error deleting song", body = super::response::delete_song::Response)
        )
    )]
    pub async fn delete_song(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::delete_song::Response>,
    ) {
        let mut response = super::response::delete_song::Response::default();

        match repo::song::get_song(&pool, &id).await {
            Ok(song) => {
                match repo::coverart::get_coverart_with_song_id(&pool, &song.id).await {
                    Ok(coverart) => {
                        let coverart_path_str = match coverart.get_path() {
                            Ok(path) => path,
                            Err(err) => {
                                response.message = err.to_string();
                                return (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    axum::Json(response),
                                );
                            }
                        };
                        let coverart_path = std::path::Path::new(&coverart_path_str);

                        if coverart_path.exists() {
                            match repo::song::delete_song(&pool, &song.id).await {
                                Ok(deleted_song) => {
                                    match repo::coverart::delete_coverart(&pool, &coverart.id).await
                                    {
                                        Ok(deleted_coverart) => {
                                            match song.remove_from_filesystem() {
                                                Ok(_) => match coverart.remove_from_filesystem() {
                                                    Ok(_) => {
                                                        response.message = String::from(
                                                            super::super::response::SUCCESSFUL,
                                                        );
                                                        response.data.push(super::response::delete_song::SongAndCoverArt{ song: deleted_song, coverart: deleted_coverart });
                                                        (
                                                            axum::http::StatusCode::OK,
                                                            axum::Json(response),
                                                        )
                                                    }
                                                    Err(err) => {
                                                        response.message = err.to_string();
                                                        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, axum::Json(response))
                                                    }
                                                },
                                                Err(err) => {
                                                    response.message = err.to_string();
                                                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, axum::Json(response))
                                                }
                                            }
                                        }

                                        Err(err) => {
                                            response.message = err.to_string();
                                            (
                                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                                axum::Json(response),
                                            )
                                        }
                                    }
                                }
                                Err(err) => {
                                    response.message = err.to_string();
                                    (
                                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                        axum::Json(response),
                                    )
                                }
                            }
                        } else {
                            response.message =
                                String::from("Could not locate coverart on the filesystem");
                            (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                axum::Json(response),
                            )
                        }
                    }
                    Err(err) => {
                        response.message = err.to_string();
                        (axum::http::StatusCode::NOT_FOUND, axum::Json(response))
                    }
                }
            }
            Err(err) => {
                response.message = err.to_string();
                (axum::http::StatusCode::NOT_FOUND, axum::Json(response))
            }
        }
    }
}
