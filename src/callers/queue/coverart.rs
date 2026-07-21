#[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct CoverArtQueue {
    pub id: uuid::Uuid,
    pub file_type: String,
    pub song_queue_id: uuid::Uuid,
}

pub mod request {
    pub mod queue {
        #[derive(utoipa::ToSchema)]
        pub struct Request {
            /// Filename
            pub file: String,
            #[schema(rename = "type")]
            /// File type. Should be a file and not a value
            pub file_type: String,
            /// Raw data of the cover art file
            pub value: Vec<u8>,
        }
    }

    pub mod link {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Request {
            pub coverart_id: uuid::Uuid,
            pub song_queue_id: uuid::Uuid,
        }
    }

    pub mod fetch_coverart_no_data {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Params {
            pub id: Option<uuid::Uuid>,
            pub song_queue_id: Option<uuid::Uuid>,
        }
    }

    pub mod fetch_coverart_with_data {
        #[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Params {
            pub id: Option<uuid::Uuid>,
            pub song_queue_id: Option<uuid::Uuid>,
        }
    }

    pub mod wipe_data_from_coverart_queue {
        #[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Request {
            pub coverart_queue_id: uuid::Uuid,
        }
    }
}

pub mod response {
    pub mod queue {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<uuid::Uuid>,
        }
    }

    pub mod link {
        #[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Id {
            pub coverart_id: uuid::Uuid,
            pub song_queue_id: uuid::Uuid,
        }

        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<Id>,
        }
    }

    pub mod fetch_coverart_no_data {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<super::super::CoverArtQueue>,
        }
    }

    pub mod fetch_coverart_with_data {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<Vec<u8>>,
        }
    }

    pub mod wipe_data_from_coverart_queue {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<uuid::Uuid>,
        }
    }
}

mod helper {
    pub fn is_coverart_file_type_valid(file_type: &String) -> bool {
        let valid_file_types = vec![
            String::from(simeta::detection::coverart::constants::JPEG_TYPE),
            String::from(simeta::detection::coverart::constants::JPG_TYPE),
            String::from(simeta::detection::coverart::constants::PNG_TYPE),
        ];

        for valid_file_type in valid_file_types {
            if valid_file_type == *file_type {
                return true;
            }
        }

        false
    }
}

pub mod endpoint {
    use axum::response::IntoResponse;

    use crate::repo::queue as repo;

    /// Endpoint to queue cover art
    #[utoipa::path(
        post,
        path = super::super::endpoints::QUEUECOVERART,
        request_body(
            content = super::request::queue::Request,
            ),
        responses(
            (status = 200, description = "Successful", body = super::response::queue::Response),
            (status = 400, description = "Error queueing cover art", body = super::response::queue::Response)
        )
    )]
    pub async fn queue(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        mut multipart: axum::extract::Multipart,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::queue::Response>,
    ) {
        let mut response = super::response::queue::Response::default();

        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap().to_string();
                let file_name = field.file_name().unwrap().to_string();
                let content_type = field.content_type().unwrap().to_string();
                let data = field.bytes().await.unwrap();
                let raw_data = data.to_vec();
                let file_type = match simeta::detection::coverart::file_type_from_data(&raw_data) {
                    Ok(file_type) => file_type,
                    Err(err) => {
                        eprintln!("Error: {err:?}");
                        response.message = err.to_string();
                        return (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            axum::Json(response),
                        );
                    }
                };

                if !super::helper::is_coverart_file_type_valid(&file_type.file_type) {
                    response.message = format!("CoverArt file type not supported: {file_type:?}");
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(response),
                    )
                } else {
                    println!(
                        "Received file '{}' (name = '{}', content-type = '{}', size = {}, file-type = {:?})",
                        file_name,
                        name,
                        content_type,
                        data.len(),
                        file_type
                    );

                    match repo::coverart::insert(&pool, &raw_data, &file_type.file_type).await {
                        Ok(id) => {
                            response.message = String::from("Successful");
                            response.data.push(id);
                            (axum::http::StatusCode::OK, axum::Json(response))
                        }
                        Err(err) => {
                            response.message = err.to_string();
                            (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                        }
                    }
                }
            }
            Ok(None) => (axum::http::StatusCode::BAD_REQUEST, axum::Json(response)),
            Err(err) => {
                response.message = err.to_string();
                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
            }
        }
    }

    /// Endpoint to link queued cover art
    #[utoipa::path(
        post,
        path = super::super::endpoints::QUEUECOVERARTLINK,
        request_body(
            content = super::request::link::Request,
            description = "Linking queued cover art to queued song",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Queued cover art linked", body = super::response::link::Response),
            (status = 400, description = "Linkage failed", body = super::response::link::Response)
        )
    )]
    pub async fn link(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::Json(payload): axum::Json<super::request::link::Request>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::link::Response>,
    ) {
        let mut response = super::response::link::Response::default();
        let id = payload.coverart_id;
        let song_id = payload.song_queue_id;

        match repo::coverart::update(&pool, &id, &song_id).await {
            Ok(_o) => {
                response.data.push(super::response::link::Id {
                    song_queue_id: song_id,
                    coverart_id: id,
                });

                (axum::http::StatusCode::OK, axum::Json(response))
            }
            Err(err) => {
                response.message = err.to_string();
                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
            }
        }
    }

    /// Endpoint to fetch cover art details
    #[utoipa::path(
        get,
        path = super::super::endpoints::QUEUECOVERART,
        params(
            ("id" = uuid::Uuid, Path, description = "Queued cover art Id"),
            ("song_queue_id" = uuid::Uuid, Path, description = "Queued song Id")
        ),
        responses(
            (status = 200, description = "Queued song linked", body = super::response::fetch_coverart_no_data::Response),
            (status = 400, description = "Linkage failed", body = super::response::fetch_coverart_no_data::Response)
        )
    )]
    pub async fn fetch_coverart_no_data(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Query(params): axum::extract::Query<
            super::request::fetch_coverart_no_data::Params,
        >,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::fetch_coverart_no_data::Response>,
    ) {
        let mut response = super::response::fetch_coverart_no_data::Response::default();

        match params.id {
            Some(id) => match repo::coverart::get_coverart_queue_with_id(&pool, &id).await {
                Ok(cover_art_queue) => {
                    response.message = String::from("Successful");
                    response.data.push(cover_art_queue);
                    (axum::http::StatusCode::OK, axum::Json(response))
                }
                Err(err) => {
                    response.message = err.to_string();
                    (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                }
            },
            _ => match params.song_queue_id {
                Some(song_queue_id) => match repo::coverart::get_coverart_queue_with_song_queue_id(
                    &pool,
                    &song_queue_id,
                )
                .await
                {
                    Ok(cover_art_queue) => {
                        response.message = String::from("Successful");
                        response.data.push(cover_art_queue);
                        (axum::http::StatusCode::OK, axum::Json(response))
                    }
                    Err(err) => {
                        response.message = err.to_string();
                        (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                    }
                },
                None => {
                    response.message = String::from("No valid id provided");
                    (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                }
            },
        }
    }

    /// Endpoint to fetch the queued cover art data
    #[utoipa::path(
        get,
        path = super::super::endpoints::QUEUECOVERARTDATA,
        params(("id" = uuid::Uuid, Path, description = "Queued cover art Id")),
        responses(
            (status = 200, description = "Queued cover art data", body = Vec<u8>),
            (status = 400, description = "Error fetching queued cover art data", body = Vec<u8>)
        )
    )]
    pub async fn fetch_coverart_with_data(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    ) -> (axum::http::StatusCode, axum::response::Response) {
        match repo::coverart::get_coverart_queue_data_with_id(&pool, &id).await {
            Ok(data) => {
                let file_type = simeta::detection::coverart::file_type_from_data(&data).unwrap();
                let bytes = axum::body::Bytes::from(data);
                let mut response = bytes.into_response();
                let headers = response.headers_mut();
                headers.insert(
                    axum::http::header::CONTENT_TYPE,
                    file_type.mime.parse().unwrap(),
                );

                let coverart_type = if file_type.file_type
                    == simeta::detection::coverart::constants::JPEG_TYPE
                {
                    simodels::types::CoverArtType::JpegExtension
                } else if file_type.file_type == simeta::detection::coverart::constants::JPG_TYPE {
                    simodels::types::CoverArtType::JpgExtension
                } else if file_type.file_type == simeta::detection::coverart::constants::PNG_TYPE {
                    simodels::types::CoverArtType::PngExtension
                } else {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        axum::response::Response::default(),
                    );
                };
                let filename = simodels::coverart::generate_filename(coverart_type, true).unwrap();
                headers.insert(
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{filename}\"")
                        .parse()
                        .unwrap(),
                );

                (axum::http::StatusCode::OK, response)
            }
            Err(_err) => (
                axum::http::StatusCode::BAD_REQUEST,
                axum::response::Response::default(),
            ),
        }
    }

    /// Endpoint to wipe data from the cover art queue
    #[utoipa::path(
        patch,
        path = super::super::endpoints::QUEUECOVERARTDATAWIPE,
        request_body(
            content = super::request::wipe_data_from_coverart_queue::Request,
            description = "Data required to wipe the data from the cover art queue",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Data wiped from cover art queue", body = super::response::wipe_data_from_coverart_queue::Response),
            (status = 400, description = "Error wiping the data", body = super::response::wipe_data_from_coverart_queue::Response),
            (status = 404, description = "Cover art not found", body = super::response::wipe_data_from_coverart_queue::Response)
        )
    )]
    pub async fn wipe_data_from_coverart_queue(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::Json(payload): axum::Json<super::request::wipe_data_from_coverart_queue::Request>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::wipe_data_from_coverart_queue::Response>,
    ) {
        let mut response = super::response::wipe_data_from_coverart_queue::Response::default();
        let coverart_queue_id = payload.coverart_queue_id;

        match repo::coverart::get_coverart_queue_with_id(&pool, &coverart_queue_id).await {
            Ok(coverart_queue) => {
                match repo::coverart::wipe_data(&pool, &coverart_queue.id).await {
                    Ok(id) => {
                        response.message = String::from("Success");
                        response.data.push(id);
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
                (axum::http::StatusCode::NOT_FOUND, axum::Json(response))
            }
        }
    }
}
