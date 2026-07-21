pub mod request {
    pub mod song_queue {
        #[derive(utoipa::ToSchema)]
        pub struct SongQueueRequest {
            /// Filename
            pub file: String,
            #[schema(rename = "type")]
            /// File type. Should be a file and not a value
            pub file_type: String,
            /// Raw data of the flac file
            pub value: Vec<u8>,
        }
    }

    pub mod update_status {
        #[derive(Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Request {
            pub id: uuid::Uuid,
            pub status: String,
        }
    }

    pub mod wipe_data_from_song_queue {
        #[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Request {
            pub song_queue_id: uuid::Uuid,
        }
    }

    pub mod link_user_id {
        #[derive(Debug, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Request {
            pub song_queue_id: uuid::Uuid,
            pub user_id: uuid::Uuid,
        }
    }
}

pub mod response {
    pub mod song_queue {
        /// Song queue response
        #[derive(Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            /// Id of the queued song
            pub data: Vec<uuid::Uuid>,
        }
    }

    pub mod fetch_queue_song {
        use serde::{Deserialize, Serialize};

        #[derive(Default, Deserialize, Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<crate::repo::queue::song::dbtype::SongQueue>,
        }
    }

    pub mod update_status {
        #[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct ChangedStatus {
            pub old_status: String,
            pub new_status: String,
        }

        #[derive(Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<ChangedStatus>,
        }
    }

    pub mod update_song_queue {
        #[derive(Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<uuid::Uuid>,
        }
    }

    pub mod wipe_data_from_song_queue {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<uuid::Uuid>,
        }
    }

    pub mod link_user_id {
        #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
        pub struct Response {
            pub message: String,
            pub data: Vec<uuid::Uuid>,
        }
    }
}

pub async fn is_song_valid(data: &[u8]) -> Result<bool, std::io::Error> {
    match simeta::detection::song::file_type_from_data(data) {
        Ok(file_type) => {
            if file_type.file_type == simeta::detection::song::constants::FLAC_TYPE {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(err) => Err(err),
    }
}

pub mod endpoint {
    use axum::response::IntoResponse;

    use crate::repo::queue as repo;

    /// Endpoint to queue a song. Starts the process and places the song in a queue
    #[utoipa::path(
        post,
        path = super::super::endpoints::QUEUESONG,
        request_body(
            content = super::request::song_queue::SongQueueRequest,
            description = "Multipart form data for uploading song",
            content_type = "multipart/form-data"
            ),
        responses(
            (status = 201, description = "Song queued", body = super::response::song_queue::Response),
            (status = 400, description = "Invalid request passed", body = super::response::song_queue::Response),
            (status = 500, description = "Error queueing song", body = super::response::song_queue::Response)
        )
    )]
    pub async fn queue_song(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        mut multipart: axum::extract::Multipart,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::song_queue::Response>,
    ) {
        let mut results: Vec<uuid::Uuid> = Vec::new();
        let mut response = super::response::song_queue::Response::default();

        match multipart.next_field().await {
            Ok(multipart_field) => {
                if let Some(field) = multipart_field {
                    let name = field.name().unwrap().to_string();
                    let file_name = field.file_name().unwrap().to_string();
                    let content_type = field.content_type().unwrap().to_string();
                    let data = field.bytes().await.unwrap();

                    println!(
                        "Received file '{}' (name = '{}', content-type = '{}', size = {})",
                        file_name,
                        name,
                        content_type,
                        data.len()
                    );

                    let raw_data: Vec<u8> = data.to_vec();
                    match super::is_song_valid(&raw_data).await {
                        Ok(valid) => {
                            if valid {
                                match repo::song::insert(
                                    &pool,
                                    &raw_data,
                                    &file_name,
                                    &crate::repo::queue::song::status::PENDING.to_string(),
                                )
                                .await
                                {
                                    Ok(queued_song) => {
                                        results.push(queued_song);
                                    }
                                    Err(err) => {
                                        response.message = err.to_string();
                                        return (
                                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                            axum::Json(response),
                                        );
                                    }
                                }
                            } else {
                                response.message = String::from("Invalid song type");
                                return (axum::http::StatusCode::BAD_REQUEST, axum::Json(response));
                            }
                        }
                        Err(err) => {
                            response.message = err.to_string();
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                axum::Json(response),
                            );
                        }
                    }

                    response.data = results;
                    response.message = if response.data.is_empty() {
                        String::from("Error")
                    } else {
                        String::from(super::super::super::response::SUCCESSFUL)
                    };
                    (axum::http::StatusCode::CREATED, axum::Json(response))
                } else {
                    response.message = String::from("No field found in multipart");
                    (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                }
            }
            Err(err) => {
                response.message = err.to_string();
                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
            }
        }
    }

    /// Endpoint to link a user id to a queued song
    #[utoipa::path(
        patch,
        path = super::super::endpoints::QUEUESONGLINKUSERID,
        request_body(
            content = super::request::link_user_id::Request,
            description = "User Id and queued song id",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Queued song linked", body = super::response::link_user_id::Response),
            (status = 400, description = "Linkage failed", body = super::response::link_user_id::Response)
        )
    )]
    pub async fn link_user_id(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::Json(payload): axum::Json<super::request::link_user_id::Request>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::link_user_id::Response>,
    ) {
        let mut response = super::response::link_user_id::Response::default();

        match repo::song::get_song_queue(&pool, &payload.song_queue_id).await {
            Ok(song_queue) => {
                match repo::song::link_user_id(&pool, &song_queue.id, &payload.user_id).await {
                    Ok(user_id) => {
                        response.message = String::from(crate::callers::response::SUCCESSFUL);
                        response.data.push(user_id);
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
                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
            }
        }
    }

    /// Endpoint to fetch the next queued song as long as it is available
    #[utoipa::path(
        get,
        path = super::super::endpoints::NEXTQUEUESONG,
        responses(
            (status = 200, description = "Queued song is present and available", body = super::response::fetch_queue_song::Response),
            (status = 400, description = "Linkage failed", body = super::response::fetch_queue_song::Response)
        )
    )]
    pub async fn fetch_queue_song(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::fetch_queue_song::Response>,
    ) {
        let mut response = super::response::fetch_queue_song::Response::default();

        match repo::song::get_most_recent_and_update(&pool).await {
            Ok(item) => {
                response.message = String::from("Successful");
                response.data.push(item);
                (axum::http::StatusCode::OK, axum::Json(response))
            }
            Err(err) => {
                response.message = err.to_string();
                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
            }
        }
    }

    /// Endpoint to download the queued song
    #[utoipa::path(
        get,
        path = super::super::endpoints::QUEUESONGDATA,
        params(("id" = uuid::Uuid, Path, description = "Queued song Id")),
        responses(
            (status = 200, description = "Queued song linked", body = Vec<u8>),
            (status = 400, description = "Linkage failed", body = Vec<u8>)
        )
    )]
    pub async fn download_queued_song(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    ) -> (axum::http::StatusCode, axum::response::Response) {
        println!("Id: {id}");

        match repo::song::get_data(&pool, &id).await {
            Ok(data) => {
                let by = axum::body::Bytes::from(data);
                let mut response = by.into_response();
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
                axum::http::StatusCode::BAD_REQUEST,
                axum::response::Response::default(),
            ),
        }
    }

    /// Endpoint to update the status of a queued song
    #[utoipa::path(
        patch,
        path = super::super::endpoints::QUEUESONG,
        request_body(
            content = super::request::update_status::Request,
            description = "Update the status of a queued song",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Status has been updated", body = super::response::update_status::Response),
            (status = 400, description = "Error updating status of queued song", body = super::response::update_status::Response)
        )
    )]
    pub async fn update_song_queue_status(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::Json(payload): axum::Json<super::request::update_status::Request>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::update_status::Response>,
    ) {
        let mut response = super::response::update_status::Response::default();

        if repo::song::status::is_valid(&payload.status).await {
            let id = payload.id;
            if !id.is_nil() {
                match repo::song::get_status_of_song_queue(&pool, &id).await {
                    Ok(old) => {
                        match repo::song::update_song_queue_status(&pool, &payload.status, &id)
                            .await
                        {
                            Ok(new) => {
                                response.message = String::from("Successful");
                                response
                                    .data
                                    .push(super::response::update_status::ChangedStatus {
                                        old_status: old,
                                        new_status: new,
                                    });
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
                        (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
                    }
                }
            } else {
                response.message = String::from("Id is nil");
                (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
            }
        } else {
            response.message = String::from("Status not valid");
            (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
        }
    }

    /// Endpoint to update the queued song data
    #[utoipa::path(
        patch,
        path = super::super::endpoints::QUEUESONGUPDATE,
        request_body(
            content = super::request::song_queue::SongQueueRequest,
            description = "Multipart form data for uploading song",
            content_type = "multipart/form-data"
            ),
        params(("id" = uuid::Uuid, Path, description = "Queued song Id")),
        responses(
            (status = 200, description = "Queued song updated", body = super::response::update_song_queue::Response),
            (status = 400, description = "Error updating queued song", body = super::response::update_song_queue::Response),
            (status = 404, description = "Queued song not found", body = super::response::update_song_queue::Response),
            (status = 500, description = "Error updating queued song", body = super::response::update_song_queue::Response)
        )
    )]
    pub async fn update_song_queue(
        axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        mut multipart: axum::extract::Multipart,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::update_song_queue::Response>,
    ) {
        let mut response = super::response::update_song_queue::Response::default();

        if let Some(field) = multipart.next_field().await.unwrap() {
            let name = field.name().unwrap().to_string();
            let file_name = field.file_name().unwrap().to_string();
            let content_type = match field.content_type() {
                Some(ct) => ct.to_string(),
                None => String::new(),
            };

            let data = field.bytes().await.unwrap();

            println!(
                "Received file '{}' (name = '{}', content-type = '{}', size = {})",
                file_name,
                name,
                content_type,
                data.len()
            );

            let raw_data: Vec<u8> = data.to_vec();
            match super::is_song_valid(&raw_data).await {
                Ok(valid) => {
                    if valid {
                        match repo::song::update(&pool, &raw_data, &id).await {
                            Ok(_) => {
                                response.message =
                                    String::from(super::super::super::response::SUCCESSFUL);
                                response.data.push(id);
                                (axum::http::StatusCode::OK, axum::Json(response))
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
                        response.message = String::from("Invalid song type");
                        (axum::http::StatusCode::BAD_REQUEST, axum::Json(response))
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
            response.message = String::from("No data provided");
            (axum::http::StatusCode::NOT_FOUND, axum::Json(response))
        }
    }

    /// Endpoint to wipe the data from a queued song
    #[utoipa::path(
        patch,
        path = super::super::endpoints::QUEUESONGDATAWIPE,
        request_body(
            content = super::request::wipe_data_from_song_queue::Request,
            description = "Pass the queued song Id to wipe the data",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Queued song data wiped", body = super::response::wipe_data_from_song_queue::Response),
            (status = 404, description = "Queued song cannot be found", body = super::response::wipe_data_from_song_queue::Response)
        )
    )]
    pub async fn wipe_data_from_song_queue(
        axum::Extension(pool): axum::Extension<sqlx::PgPool>,
        axum::Json(payload): axum::Json<super::request::wipe_data_from_song_queue::Request>,
    ) -> (
        axum::http::StatusCode,
        axum::Json<super::response::wipe_data_from_song_queue::Response>,
    ) {
        let mut response = super::response::wipe_data_from_song_queue::Response::default();
        let id = payload.song_queue_id;

        match repo::song::get_song_queue(&pool, &id).await {
            Ok(song_queue) => match repo::song::wipe_data(&pool, &song_queue.id).await {
                Ok(wiped_id) => {
                    response.message = String::from("Success");
                    response.data.push(wiped_id);

                    (axum::http::StatusCode::OK, axum::Json(response))
                }
                Err(err) => {
                    response.message = err.to_string();
                    (axum::http::StatusCode::NOT_FOUND, axum::Json(response))
                }
            },
            Err(err) => {
                response.message = err.to_string();
                (axum::http::StatusCode::NOT_FOUND, axum::Json(response))
            }
        }
    }
}
