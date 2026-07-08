pub mod request {
    pub mod create_coverart {
        #[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Request {
            pub song_id: uuid::Uuid,
            pub coverart_queue_id: uuid::Uuid,
        }
    }

    pub mod get_coverart {
        #[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Params {
            pub id: Option<uuid::Uuid>,
        }
    }
}

pub mod response {
    pub mod create_coverart {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<icarus_models::coverart::CoverArt>,
        }
    }

    pub mod get_coverart {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<icarus_models::coverart::CoverArt>,
        }
    }
}

pub mod endpoint {
    use axum::response::IntoResponse;

    use crate::repo;
    use crate::repo::queue as repo_queue;

    /// Endpoint to create cover art
    #[utoipa::path(
        post,
        path = super::super::endpoints::CREATECOVERART,
        request_body(
            content = super::request::create_coverart::Request,
            description = "Data required to create cover art",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Cover art created", body = super::response::create_coverart::Response),
            (status = 400, description = "Failure in creating cover art", body = super::response::create_coverart::Response)
        )
    )]
    pub async fn create_coverart(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::Json(payload): axum::Json<super::request::create_coverart::Request>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::create_coverart::Response>,
    ) {
        let mut response = super::response::create_coverart::Response::default();
        let id = payload.coverart_queue_id;

        match repo_queue::coverart::get_coverart_queue_data_with_id(&pool, &id).await {
            Ok(data) => {
                let song_id = payload.song_id;
                match crate::repo::song::get_song(&pool, &song_id).await {
                    Ok(song) => {
                        let directory = icarus_envy::environment::get_root_directory().value;
                        let file_type =
                            icarus_meta::detection::coverart::file_type_from_data(&data).unwrap();
                        let coverart_type = if file_type.file_type
                            == icarus_meta::detection::coverart::constants::JPEG_TYPE
                        {
                            icarus_models::types::CoverArtType::JpegExtension
                        } else if file_type.file_type
                            == icarus_meta::detection::coverart::constants::JPG_TYPE
                        {
                            icarus_models::types::CoverArtType::JpgExtension
                        } else if file_type.file_type
                            == icarus_meta::detection::coverart::constants::PNG_TYPE
                        {
                            icarus_models::types::CoverArtType::PngExtension
                        } else {
                            response.message = String::from("Invalid CoverArt type");
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                axum::Json(response),
                            );
                        };
                        let filename =
                            icarus_models::coverart::generate_filename(coverart_type, true)
                                .unwrap();

                        let mut coverart =
                            icarus_models::coverart::init::init_coverart_dir_and_filename(
                                &directory, &filename,
                            );
                        coverart.title = song.album.clone();
                        coverart.file_type = file_type.file_type;
                        coverart.data = data;

                        match coverart.save_to_filesystem() {
                            Ok(_) => {
                                match repo::coverart::create(&pool, &coverart, &song.id).await {
                                    Ok(id) => {
                                        coverart.song_id = song_id;
                                        coverart.id = id;
                                        println!("Cover Art created");

                                        response.message = String::from("Successful");
                                        response.data.push(coverart);

                                        (axum::http::StatusCode::OK, axum::Json(response))
                                    }
                                    Err(err) => {
                                        response.message = err.to_string();
                                        (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
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
                        (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                    }
                }
            }
            Err(err) => {
                response.message = err.to_string();
                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
            }
        }
    }

    /// Endpoint to get cover art with criteria
    #[utoipa::path(
        get,
        path = super::super::endpoints::GETCOVERART,
        params(
            ("id" = uuid::Uuid, Path, description = "Cover art Id")
            ),
        responses(
            (status = 200, description = "Cover art retrieved", body = super::response::get_coverart::Response),
            (status = 400, description = "Error retrieving cover art", body = super::response::get_coverart::Response)
        )
    )]
    pub async fn get_coverart(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Query(params): axum::extract::Query<super::request::get_coverart::Params>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::get_coverart::Response>,
    ) {
        let mut response = super::response::get_coverart::Response::default();

        match params.id {
            Some(id) => match repo::coverart::get_coverart(&pool, &id).await {
                Ok(coverart) => {
                    response.data.push(coverart);
                    response.message = String::from(super::super::response::SUCCESSFUL);
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

    /// Endpoint to download cover art
    #[utoipa::path(
        get,
        path = super::super::endpoints::DOWNLOADCOVERART,
        params(
            ("id" = uuid::Uuid, Path, description = "Cover art Id")
            ),
        responses(
            (status = 200, description = "Cover art downloading", body = Vec<u8>),
            (status = 404, description = "Cover art not found", body = Vec<u8>)
        )
    )]
    pub async fn download_coverart(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    ) -> (axum::http::StatusCode, axum::response::Response) {
        match repo::coverart::get_coverart(&pool, &id).await {
            Ok(coverart) => match icarus_models::coverart::io::to_data(&coverart) {
                Ok(data) => {
                    let (file_type, img_type) =
                        match icarus_meta::detection::coverart::file_type_from_data(&data) {
                            Ok(file_type) => {
                                if file_type.file_type
                                    == icarus_meta::detection::coverart::constants::JPEG_TYPE
                                {
                                    (file_type, icarus_models::types::CoverArtType::JpegExtension)
                                } else if file_type.file_type
                                    == icarus_meta::detection::coverart::constants::JPG_TYPE
                                {
                                    (file_type, icarus_models::types::CoverArtType::JpgExtension)
                                } else if file_type.file_type
                                    == icarus_meta::detection::coverart::constants::PNG_TYPE
                                {
                                    (file_type, icarus_models::types::CoverArtType::PngExtension)
                                } else {
                                    return (
                                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                        axum::response::Response::default(),
                                    );
                                }
                            }
                            Err(err) => {
                                eprintln!("Error: {err:?}");
                                return (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    axum::response::Response::default(),
                                );
                            }
                        };
                    let bytes = axum::body::Bytes::from(data);
                    let mut response = bytes.into_response();
                    let headers = response.headers_mut();
                    let filename =
                        icarus_models::coverart::generate_filename(img_type, true).unwrap();
                    headers.insert(
                        axum::http::header::CONTENT_TYPE,
                        file_type.mime.parse().unwrap(),
                    );
                    headers.insert(
                        axum::http::header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{filename}\"")
                            .parse()
                            .unwrap(),
                    );

                    (axum::http::StatusCode::OK, response)
                }
                Err(_err) => (
                    axum::http::StatusCode::NOT_FOUND,
                    axum::response::Response::default(),
                ),
            },
            Err(_err) => (
                axum::http::StatusCode::NOT_FOUND,
                axum::response::Response::default(),
            ),
        }
    }
}
