pub mod queue {
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    use simodels::playback_queue::{PlaybackQueue, PlaybackQueueItem};

    pub mod request {
        use super::{Deserialize, Serialize, ToSchema};

        pub mod create_playback_queue {
            use super::{Deserialize, Serialize, ToSchema};
            use simodels::playback_queue::{
                PLAYBACK_QUEUE_DEVICE_TYPE_BROWSER, PLAYBACK_QUEUE_DEVICE_TYPE_MOBILE,
            };

            #[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
            pub struct CreatePlaybackQueueRequest {
                pub name: String,
                pub device_type: String,
                pub user_id: uuid::Uuid,
            }

            impl CreatePlaybackQueueRequest {
                pub fn is_valid(&self) -> bool {
                    let pre_check =
                        !self.user_id.is_nil() && !self.name.is_empty() && !self.user_id.is_nil();

                    pre_check && self.valid_device_type()
                }

                fn valid_device_type(&self) -> bool {
                    matches!(
                        self.device_type.as_str(),
                        PLAYBACK_QUEUE_DEVICE_TYPE_BROWSER | PLAYBACK_QUEUE_DEVICE_TYPE_MOBILE
                    )
                }
            }
        }

        #[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
        pub struct GetPlaybackQueueParams {
            pub id: Option<uuid::Uuid>,
            pub device_type: Option<String>,
            pub user_id: Option<uuid::Uuid>,
        }

        #[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
        pub struct AddPlaybackQueueItemRequest {
            pub song_id: uuid::Uuid,
            pub playback_queue_id: uuid::Uuid,
        }

        impl AddPlaybackQueueItemRequest {
            pub fn is_valid(&self) -> bool {
                !self.song_id.is_nil() && !self.playback_queue_id.is_nil()
            }
        }

        #[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
        pub struct GetPlaybackQueueItemParams {
            pub id: Option<uuid::Uuid>,
            pub playback_queue_id: Option<uuid::Uuid>,
        }

        #[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
        pub struct ClearPlaybackQueueRequest {
            pub playback_queue_id: uuid::Uuid,
            pub playback_queue_item_id: Option<uuid::Uuid>,
        }

        impl ClearPlaybackQueueRequest {
            pub async fn is_valid(&self) -> bool {
                !&self.playback_queue_id.is_nil() || self.playback_queue_item_id.is_some()
            }
        }
    }

    pub mod response {
        use super::{Deserialize, Serialize, ToSchema};
        use super::{PlaybackQueue, PlaybackQueueItem};

        pub use AddPlaybackQueueItemResponse as GetPlaybackQueueItemResponse;
        pub use AddPlaybackQueueItemResponse as ClearPlaybackQueueResponse;

        pub mod create_playback_queue {
            use super::PlaybackQueue;
            use super::{Deserialize, Serialize, ToSchema};

            #[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
            pub struct CreatePlaybackQueueResponse {
                pub message: String,
                pub data: Vec<PlaybackQueue>,
            }
        }

        #[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
        pub struct GetPlaybackQueueResponse {
            pub message: String,
            pub data: Vec<PlaybackQueue>,
        }

        #[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
        pub struct AddPlaybackQueueItemResponse {
            pub message: String,
            pub data: Vec<PlaybackQueueItem>,
        }
    }

    pub mod endpoint {
        use axum::extract::Query;
        use axum::{Extension, Json, http};
        use http::StatusCode;

        use super::super::super::endpoints;
        use super::super::super::response as response_message;
        use super::request::create_playback_queue::CreatePlaybackQueueRequest;
        use super::request::{AddPlaybackQueueItemRequest, ClearPlaybackQueueRequest};
        use super::request::{GetPlaybackQueueItemParams, GetPlaybackQueueParams};
        use super::response::create_playback_queue::CreatePlaybackQueueResponse;
        use super::response::{
            AddPlaybackQueueItemResponse, ClearPlaybackQueueResponse, GetPlaybackQueueItemResponse,
            GetPlaybackQueueResponse,
        };
        use super::{PlaybackQueue, PlaybackQueueItem};
        use crate::repo::playback::queue as playback_repo;

        #[utoipa::path(
        post,
        path = endpoints::CREATEPLAYBACKQUEUE,
        request_body(
            content = CreatePlaybackQueueRequest,
            description = "Create playback queue for a user",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Queue already exists", body = CreatePlaybackQueueResponse),
            (status = 201, description = "Playback queue created", body = CreatePlaybackQueueResponse),
            (status = 400, description = "Bad request", body = CreatePlaybackQueueResponse),
            (status = 505, description = "Something went wrong", body = CreatePlaybackQueueResponse)
        )
    )]
        pub async fn create_playback_queue(
            Extension(pool): Extension<sqlx::PgPool>,
            Json(payload): Json<CreatePlaybackQueueRequest>,
        ) -> (StatusCode, Json<CreatePlaybackQueueResponse>) {
            let mut response = CreatePlaybackQueueResponse::default();

            if !payload.is_valid() {
                response.message = response_message::INVALID.to_string();
                (StatusCode::BAD_REQUEST, Json(response))
            } else {
                let res = match playback_repo::queue_existence_search(
                    &pool,
                    &payload.device_type,
                    &payload.user_id,
                )
                .await
                {
                    Ok(result) => result,
                    Err(err) => {
                        eprintln!("Error: {err:?}");
                        response.message = err.to_string();
                        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                    }
                };

                if res {
                    // Do not create plaback queue
                    response.message = "Already created".to_string();
                    (StatusCode::OK, Json(response))
                } else {
                    let mut playback_queue = PlaybackQueue {
                        name: payload.name,
                        device_type: payload.device_type,
                        user_id: payload.user_id,
                        ..Default::default()
                    };
                    match playback_repo::create_queue(&pool, &playback_queue).await {
                        Ok((id, date_created)) => {
                            playback_queue.id = id;
                            playback_queue.date_created = Some(date_created);
                            response.data.push(playback_queue);
                            response.message = response_message::SUCCESSFUL.to_string();

                            (StatusCode::CREATED, Json(response))
                        }
                        Err(err) => {
                            eprintln!("Error: {err:?}");
                            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                        }
                    }
                }
            }
        }

        #[utoipa::path(
            get,
            path = endpoints::GETPLAYBACKQUEUE,
            params(
                ("id" = uuid::Uuid, Path, description = "Id of playback queue"),
                ("device_type" = String, Path, description = "Device type of queue"),
                ("user_id" = uuid::Uuid, Path, description = "User Id")
                ),
            responses(
                (status = 200, description = "Playback queue found", body = GetPlaybackQueueResponse),
                (status = 400, description = "Error getting songs", body = GetPlaybackQueueResponse)
            )
        )]
        pub async fn get_playback_queue(
            Extension(pool): Extension<sqlx::PgPool>,
            Query(params): Query<GetPlaybackQueueParams>,
        ) -> (StatusCode, Json<GetPlaybackQueueResponse>) {
            let mut response = GetPlaybackQueueResponse::default();

            match params.id {
                Some(id) => match playback_repo::get(&pool, &id).await {
                    Ok(playback_queue) => {
                        response.message = response_message::SUCCESSFUL.to_string();
                        response.data.push(playback_queue);
                        (StatusCode::OK, Json(response))
                    }
                    Err(err) => {
                        eprintln!("Error: {err:?}");
                        response.message = err.to_string();
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                    }
                },
                None => match params.user_id {
                    Some(user_id) => {
                        let device_type = match params.device_type {
                            Some(d) => d,
                            None => {
                                response.message = response_message::INVALID.to_string();
                                return (StatusCode::BAD_REQUEST, Json(response));
                            }
                        };

                        let exists = match playback_repo::queue_existence_search(
                            &pool,
                            &device_type,
                            &user_id,
                        )
                        .await
                        {
                            Ok(res) => res,
                            Err(err) => {
                                eprintln!("Error: {err:?}");
                                response.message = err.to_string();
                                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                            }
                        };

                        if exists {
                            match playback_repo::get_with_device_type_and_user_id(
                                &pool,
                                &device_type,
                                &user_id,
                            )
                            .await
                            {
                                Ok(playback_queue) => {
                                    response.message = response_message::SUCCESSFUL.to_string();
                                    response.data.push(playback_queue);
                                    (StatusCode::OK, Json(response))
                                }
                                Err(err) => {
                                    eprintln!("Error: {err:?}");
                                    response.message = err.to_string();
                                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                                }
                            }
                        } else {
                            (StatusCode::OK, Json(response))
                        }
                    }
                    None => {
                        response.message = response_message::INVALID.to_string();
                        (StatusCode::BAD_REQUEST, Json(response))
                    }
                },
            }
        }

        /// Add to playback queue
        #[utoipa::path(
        post,
        path = endpoints::ADDPLAYBACKQUEUE,
        request_body(
            content = AddPlaybackQueueItemRequest,
            description = "Create playback queue item",
            content_type = "application/json"
            ),
        responses(
            (status = 200, description = "Queue item added", body = AddPlaybackQueueItemResponse),
            (status = 400, description = "Bad request", body = AddPlaybackQueueItemResponse),
            (status = 505, description = "Something went wrong", body = AddPlaybackQueueItemResponse)
        )
    )]
        pub async fn add_to_playback_queue(
            Extension(pool): Extension<sqlx::PgPool>,
            Json(payload): Json<AddPlaybackQueueItemRequest>,
        ) -> (StatusCode, Json<AddPlaybackQueueItemResponse>) {
            let mut response = AddPlaybackQueueItemResponse::default();

            if !payload.is_valid() {
                response.message = response_message::INVALID.to_string();
                return (StatusCode::BAD_REQUEST, Json(response));
            }

            let contains_items =
                match playback_repo::item::queue_items_exists(&pool, &payload.playback_queue_id)
                    .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        eprintln!("Error: {err:?}");
                        response.message = err.to_string();
                        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                    }
                };

            let queue_index = if contains_items {
                let queue_items =
                    match playback_repo::item::get_queue_items(&pool, &payload.playback_queue_id)
                        .await
                    {
                        Ok(items) => items,
                        Err(err) => {
                            eprintln!("Error: {err:?}");
                            response.message = err.to_string();
                            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                        }
                    };

                let most_recent_index = {
                    let mut index = 0;
                    for queue in &queue_items {
                        if index < queue.queue_index {
                            index = queue.queue_index;
                        }
                    }

                    index
                };
                most_recent_index + 1
            } else {
                0
            };

            let mut playback_queue_item = PlaybackQueueItem {
                queue_index,
                playback_queue_id: payload.playback_queue_id,
                song_id: payload.song_id,
                ..Default::default()
            };
            match playback_repo::item::create_queue_item(&pool, &playback_queue_item).await {
                Ok((playback_queue_item_id, date_created)) => {
                    playback_queue_item.id = playback_queue_item_id;
                    playback_queue_item.date_created = Some(date_created);
                    response.data.push(playback_queue_item);
                    response.message = response_message::SUCCESSFUL.to_string();
                    (StatusCode::OK, Json(response))
                }
                Err(err) => {
                    eprintln!("Error: {err:?}");
                    response.message = err.to_string();
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                }
            }
        }

        /// Get playback queue item
        #[utoipa::path(
        get,
        path = endpoints::GETPLAYBACKQUEUEITEM,
            params(
                ("id" = uuid::Uuid, Path, description = "Id of playback queue item"),
                ("playback_queue_id" = uuid::Uuid, Path, description = "Id of playback queue")
                ),
            responses(
                (status = 200, description = "Queue item found", body = GetPlaybackQueueItemResponse),
                (status = 400, description = "Bad request", body = GetPlaybackQueueItemResponse),
                (status = 505, description = "Something went wrong", body = GetPlaybackQueueItemResponse)
            )
        )]
        pub async fn get_playback_queue_item(
            Query(params): Query<GetPlaybackQueueItemParams>,
            Extension(pool): Extension<sqlx::PgPool>,
        ) -> (StatusCode, Json<GetPlaybackQueueItemResponse>) {
            let mut response = GetPlaybackQueueItemResponse::default();

            match params.id {
                Some(id) => match playback_repo::item::get_queue_item(&pool, &id).await {
                    Ok(playback_queue_item) => {
                        response.message = response_message::SUCCESSFUL.to_string();
                        response.data.push(playback_queue_item);
                        (StatusCode::OK, Json(response))
                    }
                    Err(err) => {
                        response.message = err.to_string();
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                    }
                },
                None => match params.playback_queue_id {
                    Some(playback_queue_id) => {
                        match playback_repo::item::get_queue_items(&pool, &playback_queue_id).await
                        {
                            Ok(playback_queue_items) => {
                                response.message = response_message::SUCCESSFUL.to_string();
                                response.data = playback_queue_items;
                                (StatusCode::OK, Json(response))
                            }
                            Err(err) => {
                                response.message = err.to_string();
                                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                            }
                        }
                    }
                    None => {
                        response.message = response_message::INVALID.to_string();
                        (StatusCode::BAD_REQUEST, Json(response))
                    }
                },
            }
        }

        /// Clear items on a playback queue
        #[utoipa::path(
            post,
            path = endpoints::CLEARPLAYBACKQUEUE,
            request_body(
                content = ClearPlaybackQueueRequest,
                description = "Clear playback queue",
                content_type = "application/json"
                ),
            responses(
                (status = 200, description = "Queue cleared", body = ClearPlaybackQueueResponse),
                (status = 400, description = "Bad request", body = ClearPlaybackQueueResponse),
                (status = 505, description = "Something went wrong", body = ClearPlaybackQueueResponse)
            )
        )]
        pub async fn clear_playback_queue(
            Extension(pool): Extension<sqlx::PgPool>,
            Json(payload): Json<ClearPlaybackQueueRequest>,
        ) -> (StatusCode, Json<ClearPlaybackQueueResponse>) {
            let mut response = ClearPlaybackQueueResponse::default();

            if !payload.is_valid().await {
                response.message = response_message::INVALID.to_string();
                (StatusCode::BAD_REQUEST, Json(response))
            } else {
                let playback_queue_id: uuid::Uuid = payload.playback_queue_id;
                match payload.playback_queue_item_id {
                    Some(playback_queue_item_id) => {
                        match playback_repo::item::get_queue_item(&pool, &playback_queue_item_id)
                            .await
                        {
                            Ok(playback_queue_item) => match playback_repo::item::clear_queue_item(
                                &pool,
                                &playback_queue_item_id,
                            )
                            .await
                            {
                                Ok(()) => {
                                    response.message = response_message::SUCCESSFUL.to_string();
                                    response.data.push(playback_queue_item);
                                }
                                Err(err) => {
                                    eprintln!("Error: {err:?}");
                                    response.message = err.to_string();
                                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                                }
                            },
                            Err(err) => {
                                eprintln!("Error: {err:?}");
                                response.message = err.to_string();
                                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                            }
                        }

                        match playback_repo::item::get_queue_items(&pool, &playback_queue_id).await
                        {
                            Ok(mut playback_queue_items) => {
                                let mut needs_to_correct: bool = false;
                                for (index, item) in
                                    &mut playback_queue_items.iter_mut().enumerate()
                                {
                                    if item.queue_index != index as i32 {
                                        item.queue_index = index as i32;
                                        needs_to_correct = true;
                                    }
                                }

                                if needs_to_correct {
                                    for item in &playback_queue_items {
                                        match playback_repo::item::update_index(
                                            &pool,
                                            &item,
                                            item.queue_index,
                                        )
                                        .await
                                        {
                                            Ok(()) => {}
                                            Err(err) => {
                                                eprintln!("Error: {err:?}");
                                                response.message = err.to_string();
                                                return (
                                                    StatusCode::INTERNAL_SERVER_ERROR,
                                                    Json(response),
                                                );
                                            }
                                        }
                                    }
                                }
                                (StatusCode::OK, Json(response))
                            }
                            Err(err) => {
                                response.message = err.to_string();
                                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                            }
                        }
                    }
                    None => {
                        let playback_queue_items = match playback_repo::item::get_queue_items(
                            &pool,
                            &payload.playback_queue_id,
                        )
                        .await
                        {
                            Ok(items) => items,
                            Err(err) => {
                                eprintln!("Error: {err:?}");
                                response.message = err.to_string();
                                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                            }
                        };

                        match playback_repo::item::clear_queue_items(
                            &pool,
                            &payload.playback_queue_id,
                        )
                        .await
                        {
                            Ok(()) => {
                                response.message = response_message::SUCCESSFUL.to_string();
                                response.data = playback_queue_items;
                                (StatusCode::OK, Json(response))
                            }
                            Err(err) => {
                                response.message = err.to_string();
                                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                            }
                        }
                    }
                }
            }
        }
    }
}
