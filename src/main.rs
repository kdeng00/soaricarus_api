#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    match tokio::net::TcpListener::bind(soaricarus_api::config::host::get_full()).await {
        Ok(listener) => {
            // build our application with routes
            let app = soaricarus_api::config::init::app().await;
            axum::serve(listener, app).await.unwrap();
        }
        Err(err) => {
            eprintln!("Error: {err:?}")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tower::ServiceExt;

    use soaricarus_api::db;

    mod db_mgr {
        use std::str::FromStr;

        pub const LIMIT: usize = 6;

        pub async fn get_pool() -> Result<sqlx::PgPool, sqlx::Error> {
            let tm_db_url = sienvy::environment::get_db_url().value;
            let tm_options = sqlx::postgres::PgConnectOptions::from_str(&tm_db_url).unwrap();
            sqlx::PgPool::connect_with(tm_options).await
        }

        pub async fn generate_db_name() -> String {
            let db_name = get_database_name().await.unwrap()
                + &"_"
                + &uuid::Uuid::new_v4().to_string()[..LIMIT];
            db_name
        }

        pub async fn connect_to_db(db_name: &str) -> Result<sqlx::PgPool, sqlx::Error> {
            let db_url = sienvy::environment::get_db_url().value;
            let options = sqlx::postgres::PgConnectOptions::from_str(&db_url)?.database(db_name);
            sqlx::PgPool::connect_with(options).await
        }

        pub async fn create_database(
            template_pool: &sqlx::PgPool,
            db_name: &str,
        ) -> Result<(), sqlx::Error> {
            let create_query = format!("CREATE DATABASE {}", db_name);
            match sqlx::query(sqlx::AssertSqlSafe(create_query))
                .execute(template_pool)
                .await
            {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }

        // Function to drop a database
        pub async fn drop_database(
            template_pool: &sqlx::PgPool,
            db_name: &str,
        ) -> Result<(), sqlx::Error> {
            let drop_query = format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", db_name);
            sqlx::query(sqlx::AssertSqlSafe(drop_query))
                .execute(template_pool)
                .await?;
            Ok(())
        }

        pub async fn get_database_name() -> Result<String, Box<dyn std::error::Error>> {
            let database_url = sienvy::environment::get_db_url().value;
            let parsed_url = url::Url::parse(&database_url)?;

            if parsed_url.scheme() == "postgres" || parsed_url.scheme() == "postgresql" {
                match parsed_url
                    .path_segments()
                    .and_then(|segments| segments.last().map(|s| s.to_string()))
                {
                    Some(sss) => Ok(sss),
                    None => Err("Error parsing".into()),
                }
            } else {
                // Handle other database types if needed
                Err("Error parsing".into())
            }
        }

        pub async fn migrations(pool: &sqlx::PgPool) {
            // Run migrations using the sqlx::migrate! macro
            // Assumes your test migrations are in a ./test_migrations folder relative to Cargo.toml
            sqlx::migrate!("./test_migrations")
                .run(pool)
                .await
                .expect("Failed to run migrations");
        }
    }

    mod init {
        use std::time::Duration;

        pub async fn app(pool: sqlx::PgPool) -> axum::Router {
            soaricarus_api::config::init::routes()
                .await
                .layer(axum::Extension(pool))
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024))
                .layer(tower_http::timeout::TimeoutLayer::with_status_code(
                    axum::http::StatusCode::OK,
                    Duration::from_secs(300),
                ))
        }
    }

    mod util {
        pub async fn resp_to_bytes(
            response: axum::response::Response,
        ) -> Result<axum::body::Bytes, axum::Error> {
            axum::body::to_bytes(response.into_body(), std::usize::MAX).await
        }

        pub async fn get_resp_data<Data>(response: axum::response::Response) -> Data
        where
            Data: for<'a> serde::Deserialize<'a>,
        {
            let body = resp_to_bytes(response).await.unwrap();
            serde_json::from_slice(&body).unwrap()
        }

        pub async fn format_url_with_value(endpoint: &str, value: &uuid::Uuid) -> String {
            let last = endpoint.len() - 5;
            format!("{}/{value}", &endpoint[0..last])
        }
    }

    pub fn token_fields() -> (String, String, String) {
        (
            String::from("What a twist!"),
            String::from("soaricarus_test"),
            String::from("soaricarus"),
        )
    }

    pub const TEST_USER_ID: uuid::Uuid = uuid::uuid!("cc938368-615a-4694-b2ca-6e122fa31c52");

    pub async fn test_token() -> Result<String, josekit::JoseError> {
        let key: String = sienvy::environment::get_secret_main_key().value;
        let (message, issuer, audience) = token_fields();

        let token_resource = simodels::token::TokenResource {
            message: message,
            issuer: issuer,
            audiences: vec![audience],
            id: TEST_USER_ID,
        };

        match simodels::token::create_token(&key, &token_resource, time::Duration::hours(1)) {
            Ok((access_token, _some_time)) => Ok(access_token),
            Err(err) => Err(err),
        }
    }

    pub async fn bearer_auth() -> String {
        let token = match test_token().await {
            Ok(access_token) => access_token,
            Err(err) => {
                assert!(false, "Error: {err:?}");
                String::new()
            }
        };

        format!("Bearer {token}")
    }

    mod request {
        use common_multipart_rfc7578::client::multipart::{
            Body as MultipartBody, Form as MultipartForm,
        };
        use tower::ServiceExt;

        pub async fn song_queue_req(
            app: &axum::Router,
        ) -> Result<axum::response::Response, axum::http::Error> {
            // Create multipart form
            match run_post(
                Some(ReqBody::Multipart((
                    "flac".to_string(),
                    "tests/I/track01.flac".to_string(),
                ))),
                soaricarus_api::callers::queue::endpoints::QUEUESONG,
                axum::http::Method::POST,
                true,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn song_queue_link_req(
            app: &axum::Router,
            song_queue_id: &uuid::Uuid,
            user_id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let payload =
                super::payload_data::link_user_to_queued_song(song_queue_id, user_id).await;

            match run_post(
                Some(ReqBody::Json(payload)),
                soaricarus_api::callers::queue::endpoints::QUEUESONGLINKUSERID,
                axum::http::Method::PATCH,
                true,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn fetch_queue_req(
            app: &axum::Router,
        ) -> Result<axum::response::Response, axum::http::Error> {
            match run_post(
                None,
                soaricarus_api::callers::queue::endpoints::NEXTQUEUESONG,
                axum::http::Method::GET,
                false,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn fetch_metadata_queue_req(
            app: &axum::Router,
            id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let uri = format!(
                "{}?id={}",
                soaricarus_api::callers::queue::endpoints::QUEUEMETADATA,
                id
            );

            match run_post(None, &uri, axum::http::Method::GET, false).await {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn fetch_queue_data_req(
            app: &axum::Router,
            id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let raw_uri = String::from(soaricarus_api::callers::queue::endpoints::QUEUESONGDATA);
            let end_index = raw_uri.len() - 4;
            let mut uri: String = (&raw_uri[..end_index]).to_string();
            uri += &id.to_string();

            match run_post(None, &uri, axum::http::Method::GET, false).await {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn upload_coverart_queue_req(
            app: &axum::Router,
        ) -> Result<axum::response::Response, axum::http::Error> {
            match run_post(
                Some(ReqBody::Multipart((
                    simeta::detection::coverart::constants::JPEG_TYPE.to_string(),
                    "tests/I/Coverart-1.jpg".to_string(),
                ))),
                soaricarus_api::callers::queue::endpoints::QUEUECOVERART,
                axum::http::Method::POST,
                true,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn queue_metadata_req(
            app: &axum::Router,
            song_queue_id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let payload = super::payload_data::queue_metadata(&song_queue_id).await;

            match run_post(
                Some(ReqBody::Json(payload)),
                soaricarus_api::callers::queue::endpoints::QUEUEMETADATA,
                axum::http::Method::POST,
                true,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn coverart_queue_song_queue_link_req(
            app: &axum::Router,
            coverart_id: &uuid::Uuid,
            song_queue_id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let payload = super::payload_data::link_queued_coverart_to_queued_song(
                song_queue_id,
                coverart_id,
            )
            .await;

            match run_post(
                Some(ReqBody::Json(payload)),
                soaricarus_api::callers::queue::endpoints::QUEUECOVERARTLINK,
                axum::http::Method::PATCH,
                true,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn create_coverart_req(
            app: &axum::Router,
            song_id: &uuid::Uuid,
            coverart_id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let payload = super::payload_data::create_coverart(song_id, coverart_id).await;

            match run_post(
                Some(ReqBody::Json(payload)),
                soaricarus_api::callers::endpoints::CREATECOVERART,
                axum::http::Method::POST,
                true,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn create_song_req(
            app: &axum::Router,
            song_queue_id: &uuid::Uuid,
            user_id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let payload = super::payload_data::create_song(song_queue_id, user_id).await;

            match run_post(
                Some(ReqBody::Json(payload)),
                soaricarus_api::callers::endpoints::CREATESONG,
                axum::http::Method::POST,
                true,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn update_song_queue_status_req(
            app: &axum::Router,
            song_queue_id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let payload =
                super::payload_data::update_song_queue_status_to_ready(song_queue_id).await;

            match run_post(
                Some(ReqBody::Json(payload)),
                soaricarus_api::callers::queue::endpoints::QUEUESONG,
                axum::http::Method::PATCH,
                true,
            )
            .await
            {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub async fn get_queued_coverart(
            app: &axum::Router,
            coverart_queue_id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            let uri = format!(
                "{}?id={}",
                soaricarus_api::callers::queue::endpoints::QUEUECOVERART,
                coverart_queue_id
            );

            match run_post(None, &uri, axum::http::Method::GET, false).await {
                Ok(request) => match app.clone().oneshot(request).await {
                    Ok(response) => Ok(response),
                    Err(err) => Err(axum::http::Error::from(err)),
                },
                Err(err) => Err(err),
            }
        }

        pub enum ReqBody {
            Json(serde_json::Value),
            Multipart((String, String)),
        }

        pub async fn run_post(
            payload: Option<ReqBody>,
            uri: &str,
            method: axum::http::Method,
            has_body: bool,
        ) -> Result<axum::http::Request<axum::body::Body>, axum::http::Error> {
            let mut content_type = "application/json; charset=utf-8".to_string();
            let body = if has_body {
                assert_eq!(
                    true,
                    payload.is_some(),
                    "Has request body and payload has data"
                );

                match payload {
                    Some(p) => match p {
                        ReqBody::Json(val) => axum::body::Body::from(val.to_string()),
                        ReqBody::Multipart((t, p)) => {
                            let mut form = MultipartForm::default();
                            let _ = form.add_file(t, p);

                            content_type = form.content_type();
                            let body = MultipartBody::from(form);
                            axum::body::Body::from_stream(body)
                        }
                    },
                    None => {
                        eprintln!("This should not empty");
                        axum::body::Body::empty()
                    }
                }
            } else {
                axum::body::Body::empty()
            };
            match axum::http::Request::builder()
                .method(method)
                .uri(uri)
                .header(axum::http::header::CONTENT_TYPE, content_type)
                .header(
                    axum::http::header::AUTHORIZATION,
                    super::bearer_auth().await,
                )
                .body(body)
            {
                Ok(t) => Ok(t),
                Err(err) => Err(err),
            }
        }
    }

    mod sequence_flow {
        pub const TEST_SONG_01_FILE_KEY: &str = "processed/song/track01.flac";
        pub const TEST_SONG_02_FILE_KEY: &str = "processed/song/track02.flac";

        pub struct SongBucket {
            pub file_key: String,
            pub path: String,
        }

        pub async fn upload_test_songs_to_bucket() {
            let songs = vec![
                SongBucket {
                    file_key: TEST_SONG_01_FILE_KEY.to_string(),
                    path: "tests/I/track01.flac".to_string(),
                },
                SongBucket {
                    file_key: TEST_SONG_02_FILE_KEY.to_string(),
                    path: "tests/I/track02.flac".to_string(),
                },
            ];

            let lab_config = soaricarus_api::util::maze::get_config();
            let lr = labyrinth::Labyrinth { config: lab_config };

            for song in &songs {
                let p = std::path::Path::new(&song.path);
                match tokio::fs::File::open(p).await {
                    Ok(mut _file) => {
                        let data = labyrinth::Data {
                            filepath: song.path.clone(),
                            ..Default::default()
                        };

                        match lr.upload(&song.file_key, &data).await {
                            Ok(_) => {}
                            Err(err) => {
                                assert!(false, "Error: {err:?}");
                            }
                        }
                    }
                    Err(err) => {
                        assert!(false, "Error: {err:?}");
                    }
                };
            }
        }

        pub async fn delete_test_songs_from_bucket() {
            let songs = vec![
                SongBucket {
                    file_key: TEST_SONG_01_FILE_KEY.to_string(),
                    path: "tests/I/track01.flac".to_string(),
                },
                SongBucket {
                    file_key: TEST_SONG_02_FILE_KEY.to_string(),
                    path: "tests/I/track02.flac".to_string(),
                },
            ];

            let lab_config = soaricarus_api::util::maze::get_config();
            let lr = labyrinth::Labyrinth { config: lab_config };

            for song in &songs {
                match lr.delete(&song.file_key).await {
                    Ok(_) => {}
                    Err(err) => {
                        assert!(false, "Error: {err:?}");
                    }
                }
            }
        }

        // Flow for queueing song
        pub async fn queue_song_flow(
            app: &axum::Router,
        ) -> Result<(axum::response::Response, uuid::Uuid), axum::http::Error> {
            upload_test_songs_to_bucket().await;
            match super::request::song_queue_req(&app).await {
                Ok(response) => {
                    let resp = super::util::get_resp_data::<
                        soaricarus_api::callers::queue::song::response::song_queue::Response,
                    >(response)
                    .await;
                    assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                    assert_eq!(false, resp.data[0].is_nil(), "Should not be empty");
                    let song_queue_id = resp.data[0];
                    assert_eq!(false, song_queue_id.is_nil(), "Should not be empty");

                    let user_id = super::TEST_USER_ID;

                    match super::request::song_queue_link_req(&app, &song_queue_id, &user_id).await
                    {
                        Ok(response) => {
                            let resp = super::util::get_resp_data::<
                                soaricarus_api::callers::queue::song::response::link_user_id::Response,
                            >(response)
                            .await;
                            assert_eq!(
                                false,
                                resp.data.is_empty(),
                                "The response should not be empty"
                            );

                            match super::request::queue_metadata_req(&app, &song_queue_id).await {
                                Ok(response) => {
                                    let resp = super::util::get_resp_data::<
                                        soaricarus_api::callers::queue::song::response::song_queue::Response,
                                    >(response)
                                    .await;
                                    assert_eq!(false, resp.data.is_empty(), "Should not be empty");

                                    let id = resp.data[0];

                                    match super::request::fetch_metadata_queue_req(&app, &id).await
                                    {
                                        Ok(response) => Ok((response, user_id)),
                                        Err(err) => Err(err),
                                    }
                                }
                                Err(err) => Err(err),
                            }
                        }
                        Err(err) => Err(err),
                    }
                }
                Err(err) => Err(err),
            }
        }

        // Flow for queueing coverart
        pub async fn queue_coverart_flow(
            app: &axum::Router,
            song_queue_id: &uuid::Uuid,
        ) -> Result<axum::response::Response, axum::http::Error> {
            match super::request::upload_coverart_queue_req(&app).await {
                Ok(response) => {
                    let resp = super::util::get_resp_data::<
                        soaricarus_api::callers::queue::coverart::response::queue::Response,
                    >(response)
                    .await;
                    assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                    let coverart_id = resp.data[0];
                    assert_eq!(false, coverart_id.is_nil(), "Should not be empty");

                    match super::request::coverart_queue_song_queue_link_req(
                        &app,
                        &coverart_id,
                        &song_queue_id,
                    )
                    .await
                    {
                        Ok(response) => {
                            let resp = super::util::get_resp_data::<
                                soaricarus_api::callers::queue::coverart::response::link::Response,
                            >(response)
                            .await;
                            assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                            let resp_coverart_id = resp.data[0].coverart_id;
                            let resp_song_queue_id = resp.data[0].song_queue_id;

                            assert_eq!(false, resp_coverart_id.is_nil(), "Should not be empty");
                            assert_eq!(false, resp_song_queue_id.is_nil(), "Should not be empty");

                            match super::request::get_queued_coverart(&app, &resp_coverart_id).await
                            {
                                Ok(response) => Ok(response),
                                Err(err) => Err(err),
                            }
                        }
                        Err(err) => Err(err),
                    }
                }
                Err(err) => Err(err),
            }
        }

        // Returns coverart response and song_queue_id
        pub async fn queue_song_and_coverart_flow(
            app: &axum::Router,
        ) -> Result<(axum::response::Response, uuid::Uuid), axum::http::Error> {
            match queue_song_flow(&app).await {
                Ok((song_response, user_id)) => {
                    let resp = super::util::get_resp_data::<
                        soaricarus_api::callers::queue::metadata::response::fetch_metadata::Response,
                    >(song_response)
                    .await;
                    assert_eq!(false, resp.data.is_empty(), "Data should not be empty");
                    let song_queue_id = resp.data[0].song_queue_id;

                    match super::request::create_song_req(&app, &song_queue_id, &user_id).await {
                        Ok(response) => {
                            let resp = super::util::get_resp_data::<
                                soaricarus_api::callers::song::response::create_metadata::Response,
                            >(response)
                            .await;
                            assert_eq!(
                                false,
                                resp.data.is_empty(),
                                "No songs found, Response {:?}",
                                resp
                            );
                            let song = &resp.data[0];
                            let song_id = song.id;
                            assert_eq!(
                                false,
                                song_id.is_nil(),
                                "Song id should not be nil {:?}",
                                song
                            );

                            match queue_coverart_flow(&app, &song_queue_id).await {
                                Ok(response) => Ok((response, song_queue_id)),
                                Err(err) => {
                                    assert!(false, "Error: {:?}", err);
                                    Err(err)
                                }
                            }
                        }
                        Err(err) => {
                            assert!(false, "Error: {:?}", err);
                            Err(err)
                        }
                    }
                }
                Err(err) => {
                    assert!(false, "Error: {:?}", err);
                    Err(err)
                }
            }
        }
    }

    pub mod payload_data {
        pub async fn queue_metadata(song_queue_id: &uuid::Uuid) -> serde_json::Value {
            serde_json::json!(
            {
                    "song_queue_id": song_queue_id,
                    "album" : "I",
                    "album_artist" : "Kuoth",
                    "artist" : "Kuoth",
                    "disc" : 1,
                    "disc_count" : 1,
                    "duration" : 139,
                    "genre" : "Alternative Hip-Hop",
                    "title" : "Hypocrite Like The Rest",
                    "track" : 1,
                    "track_count" : 9,
                    "year" : 2020
            })
        }

        pub async fn create_song(
            song_queue_id: &uuid::Uuid,
            user_id: &uuid::Uuid,
        ) -> serde_json::Value {
            serde_json::json!({
                "title" : "Hypocrite Like The Rest",
                "artist" : "Kuoth",
                "album_artist": "Kuoth",
                "album": "I",
                "genre" : "Alternative Hip-Hop",
                "date": "2020-01-01",
                "track": 1,
                "disc": 1,
                "track_count": 9,
                "disc_count": 1,
                "duration": 139,
                "audio_type": "flac",
                "user_id": user_id,
                "song_queue_id": song_queue_id
            })
        }

        pub async fn link_user_to_queued_song(
            song_queue_id: &uuid::Uuid,
            user_id: &uuid::Uuid,
        ) -> serde_json::Value {
            serde_json::json!({
                "song_queue_id": song_queue_id,
                "user_id": user_id
            })
        }

        pub async fn link_queued_coverart_to_queued_song(
            song_queue_id: &uuid::Uuid,
            coverart_queue_id: &uuid::Uuid,
        ) -> serde_json::Value {
            serde_json::json!({
                "song_queue_id": song_queue_id,
                "coverart_id": coverart_queue_id
            })
        }

        pub async fn create_coverart(
            song_id: &uuid::Uuid,
            coverart_queue_id: &uuid::Uuid,
        ) -> serde_json::Value {
            serde_json::json!({
                "song_id": song_id,
                "coverart_queue_id": coverart_queue_id
            })
        }

        pub async fn update_song_queue_status_to_ready(
            song_queue_id: &uuid::Uuid,
        ) -> serde_json::Value {
            serde_json::json!({
                "id": song_queue_id,
                "status": soaricarus_api::repo::queue::song::status::READY
            })
        }
    }

    #[tokio::test]
    async fn test_song_queue() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match request::song_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::song::response::song_queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                assert_eq!(false, resp.data[0].is_nil(), "Should not be empty");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
        sequence_flow::delete_test_songs_from_bucket().await;
    }

    #[tokio::test]
    async fn test_song_queue_link_user_id() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        match request::song_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::song::response::song_queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                assert_eq!(false, resp.data[0].is_nil(), "Should not be empty");

                let song_queue_id = &resp.data[0];
                let user_id = TEST_USER_ID;
                println!("User Id: {user_id:?}");

                match request::song_queue_link_req(&app, &song_queue_id, &user_id).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::queue::song::response::link_user_id::Response,
                        >(response)
                        .await;
                        let collected_user_id = &resp.data[0];

                        assert!(
                            !collected_user_id.is_nil(),
                            "Collected user id should not be nil {collected_user_id:?}"
                        );
                        assert_eq!(
                            user_id, *collected_user_id,
                            "User Id is different. First {user_id:?} Second {collected_user_id:?}"
                        );
                    }
                    Err(err) => {
                        assert!(
                            false,
                            "Error: {err:?} songQueue Id {song_queue_id:?} user id {user_id:?}"
                        );
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_song_fetch_queue_item() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        match sequence_flow::queue_song_and_coverart_flow(&app).await {
            Ok((resp_one, song_queue_id)) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::coverart::response::fetch_coverart_no_data::Response,
                >(resp_one)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");

                let _resp_coverart_queue_id = resp.data[0].id;

                let old = soaricarus_api::repo::queue::song::status::PENDING;
                let target_status = soaricarus_api::repo::queue::song::status::READY;

                match request::update_song_queue_status_req(&app, &song_queue_id).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::queue::song::response::update_status::Response,
                        >(response)
                        .await;
                        assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                        let changed_status = &resp.data[0];

                        assert_eq!(*old, changed_status.old_status, "Old status does not match");
                        assert_eq!(
                            target_status, changed_status.new_status,
                            "New status does not match"
                        );

                        match request::fetch_queue_req(&app).await {
                            Ok(response) => {
                                let resp = util::get_resp_data::<
                                    soaricarus_api::callers::queue::song::response::fetch_queue_song::Response,
                                >(response)
                                .await;
                                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                            }
                            Err(err) => {
                                assert!(false, "Error: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_update_song_from_queue() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match request::song_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::song::response::song_queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                assert_eq!(false, resp.data[0].is_nil(), "Should not be empty");

                let id = &resp.data[0];

                match request::fetch_queue_data_req(&app, &id).await {
                    Ok(response) => match util::resp_to_bytes(response).await {
                        Ok(bytes) => {
                            assert_eq!(false, bytes.is_empty(), "Queued data should not be empty");

                            let temp_file =
                                tempfile::tempdir().expect("Could not create test directory");
                            let test_dir = String::from(temp_file.path().to_str().unwrap());
                            let song = simodels::song::Song {
                                directory: test_dir,
                                filename: simodels::song::generate_filename(
                                    simodels::types::MusicType::FlacExtension,
                                    true,
                                )
                                .unwrap(),
                                data: bytes.to_vec(),
                                ..Default::default()
                            };
                            match song.save_to_filesystem() {
                                Ok(_) => {}
                                Err(err) => {
                                    assert!(false, "Error: {err:?}")
                                }
                            }
                            let songpath = song.song_path().unwrap();

                            let raw_uri = String::from(
                                soaricarus_api::callers::queue::endpoints::QUEUESONGUPDATE,
                            );
                            let end_index = raw_uri.len() - 5;

                            let uri = format!(
                                "{}/{}",
                                (&raw_uri[..end_index]).to_string(),
                                id.to_string()
                            );

                            match app
                                .clone()
                                .oneshot(
                                    request::run_post(
                                        Some(request::ReqBody::Multipart((
                                            "flac".to_string(),
                                            songpath,
                                        ))),
                                        &uri,
                                        axum::http::Method::PATCH,
                                        true,
                                    )
                                    .await
                                    .unwrap(),
                                )
                                .await
                            {
                                Ok(response) => {
                                    let resp = util::get_resp_data::<
                                        soaricarus_api::callers::queue::song::response::update_song_queue::Response,
                                    >(response)
                                    .await;
                                    assert_eq!(false, resp.data.is_empty(), "Should not be empty");

                                    let updated_song_queued_id = resp.data[0];
                                    assert_eq!(
                                        updated_song_queued_id, *id,
                                        "Song queue Id should match, but they don't. {:?} {:?}",
                                        updated_song_queued_id, id
                                    );
                                }
                                Err(err) => {
                                    assert!(false, "Error: {:?}", err);
                                }
                            }
                        }
                        Err(err) => {
                            assert!(false, "Error: {:?}", err);
                        }
                    },
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_song_fetch_queue_data() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match request::song_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::song::response::song_queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                assert_eq!(false, resp.data[0].is_nil(), "Should not be empty");
                let id = resp.data[0];

                match request::fetch_queue_data_req(&app, &id).await {
                    Ok(response) => match util::resp_to_bytes(response).await {
                        Ok(bytes) => {
                            assert_eq!(false, bytes.is_empty(), "Queued data should not be empty");
                        }
                        Err(err) => {
                            assert!(false, "Error: {:?}", err);
                        }
                    },
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_song_queue_update_status() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        match sequence_flow::queue_song_and_coverart_flow(&app).await {
            Ok((resp_one, song_queue_id)) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::coverart::response::fetch_coverart_no_data::Response,
                >(resp_one)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");

                let _resp_coverart_queue_id = resp.data[0].id;

                let old = soaricarus_api::repo::queue::song::status::PENDING;
                let done = soaricarus_api::repo::queue::song::status::READY;

                match request::update_song_queue_status_req(&app, &song_queue_id).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::queue::song::response::update_status::Response,
                        >(response)
                        .await;
                        assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                        let changed_status = &resp.data[0];

                        assert_eq!(*old, changed_status.old_status, "Old status does not match");
                        assert_eq!(done, changed_status.new_status, "New status does not match");
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_song_metadata_queue() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match request::song_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::song::response::song_queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                assert_eq!(false, resp.data[0].is_nil(), "Should not be empty");

                match request::queue_metadata_req(&app, &resp.data[0]).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::queue::song::response::song_queue::Response,
                        >(response)
                        .await;
                        assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_get_metadata_queue() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match sequence_flow::queue_song_flow(&app).await {
            Ok((response, _user_id)) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::metadata::response::fetch_metadata::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Data should not be empty");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_song_coverart_queue() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match request::upload_coverart_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::coverart::response::queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                let id = resp.data[0];
                assert_eq!(false, id.is_nil(), "Should not be empty");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_song_coverart_queue_link() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        match request::song_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::coverart::response::queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                let song_queue_id = resp.data[0];
                assert_eq!(false, song_queue_id.is_nil(), "Should not be empty");

                // Send request
                match request::upload_coverart_queue_req(&app).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::queue::coverart::response::queue::Response,
                        >(response)
                        .await;
                        assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                        let coverart_id = resp.data[0];
                        assert_eq!(false, coverart_id.is_nil(), "Should not be empty");

                        match request::coverart_queue_song_queue_link_req(
                            &app,
                            &coverart_id,
                            &song_queue_id,
                        )
                        .await
                        {
                            Ok(response) => {
                                let resp = util::get_resp_data::<
                                    soaricarus_api::callers::queue::coverart::response::link::Response,
                                >(response)
                                .await;
                                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                                let resp_coverart_id = resp.data[0].coverart_id;
                                let resp_song_queue_id = resp.data[0].song_queue_id;

                                assert_eq!(false, resp_coverart_id.is_nil(), "Should not be empty");
                                assert_eq!(
                                    false,
                                    resp_song_queue_id.is_nil(),
                                    "Should not be empty"
                                );
                            }
                            Err(err) => {
                                assert!(false, "Error: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_fetch_coverart_queue_without_data() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        match request::song_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::coverart::response::queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                let song_queue_id = resp.data[0];
                assert_eq!(false, song_queue_id.is_nil(), "Should not be empty");

                match sequence_flow::queue_coverart_flow(&app, &song_queue_id).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::queue::coverart::response::fetch_coverart_no_data::Response,
                        >(response)
                        .await;
                        assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_fetch_coverart_queue_with_data() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        match request::song_queue_req(&app).await {
            Ok(response) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::coverart::response::queue::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                let song_queue_id = resp.data[0];
                assert_eq!(false, song_queue_id.is_nil(), "Should not be empty");

                // Send request
                match request::upload_coverart_queue_req(&app).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::queue::coverart::response::queue::Response,
                        >(response)
                        .await;
                        assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                        let coverart_id = resp.data[0];
                        assert_eq!(false, coverart_id.is_nil(), "Should not be empty");

                        match request::coverart_queue_song_queue_link_req(
                            &app,
                            &coverart_id,
                            &song_queue_id,
                        )
                        .await
                        {
                            Ok(response) => {
                                let resp = util::get_resp_data::<
                                    soaricarus_api::callers::queue::coverart::response::link::Response,
                                >(response)
                                .await;
                                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                                let resp_coverart_id = resp.data[0].coverart_id;
                                let resp_song_queue_id = resp.data[0].song_queue_id;

                                assert_eq!(false, resp_coverart_id.is_nil(), "Should not be empty");
                                assert_eq!(
                                    false,
                                    resp_song_queue_id.is_nil(),
                                    "Should not be empty"
                                );

                                let raw_uri = String::from(
                                    soaricarus_api::callers::queue::endpoints::QUEUECOVERARTDATA,
                                );
                                let end_index = raw_uri.len() - 5;
                                let uri = format!(
                                    "{}/{}",
                                    (&raw_uri[..end_index]).to_string(),
                                    resp_coverart_id
                                );
                                println!("Uri: {:?}", uri);

                                match app
                                    .clone()
                                    .oneshot(
                                        request::run_post(
                                            None,
                                            &uri,
                                            axum::http::Method::GET,
                                            false,
                                        )
                                        .await
                                        .unwrap(),
                                    )
                                    .await
                                {
                                    Ok(response) => match util::resp_to_bytes(response).await {
                                        Ok(bytes) => {
                                            assert_eq!(
                                                false,
                                                bytes.is_empty(),
                                                "Downloaded coverart data should not be empty"
                                            );
                                            let temp_file = tempfile::tempdir()
                                                .expect("Could not create test directory");
                                            let test_dir =
                                                String::from(temp_file.path().to_str().unwrap());
                                            let new_file = format!("{}/new_image.jpeg", test_dir);

                                            let mut file =
                                                std::fs::File::create(&new_file).unwrap();
                                            file.write_all(&bytes).unwrap();
                                        }
                                        Err(err) => {
                                            assert!(false, "Error: {:?}", err);
                                        }
                                    },
                                    Err(err) => {
                                        assert!(false, "Error: {:?}", err);
                                    }
                                }
                            }
                            Err(err) => {
                                assert!(false, "Error: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_create_song() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match sequence_flow::queue_song_flow(&app).await {
            Ok((response, user_id)) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::metadata::response::fetch_metadata::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Data should not be empty");
                let song_q_id = resp.data[0].song_queue_id;

                match request::create_song_req(&app, &song_q_id, &user_id).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::song::response::create_metadata::Response,
                        >(response)
                        .await;
                        assert_eq!(
                            false,
                            resp.data.is_empty(),
                            "No songs found, Response {:?}",
                            resp
                        );
                        let song = &resp.data[0];
                        let song_id = song.id;
                        assert_eq!(
                            false,
                            song_id.is_nil(),
                            "Song id should not be nil {:?}",
                            song
                        );
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_create_coverart() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match sequence_flow::queue_song_flow(&app).await {
            Ok((response, user_id)) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::metadata::response::fetch_metadata::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Data should not be empty");
                let song_queue_id = resp.data[0].song_queue_id;

                match request::create_song_req(&app, &song_queue_id, &user_id).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::song::response::create_metadata::Response,
                        >(response)
                        .await;
                        assert_eq!(
                            false,
                            resp.data.is_empty(),
                            "No songs found, Response {:?}",
                            resp
                        );
                        let song = &resp.data[0];
                        let song_id = song.id;
                        assert_eq!(
                            false,
                            song_id.is_nil(),
                            "Song id should not be nil {:?}",
                            song
                        );

                        match sequence_flow::queue_coverart_flow(&app, &song_queue_id).await {
                            Ok(response) => {
                                let resp = util::get_resp_data::<
                                                    soaricarus_api::callers::queue::coverart::response::fetch_coverart_no_data::Response,
                                                >(response)
                                                .await;
                                assert_eq!(false, resp.data.is_empty(), "Should not be empty");
                                let resp_queue_coverart_id = resp.data[0].id;

                                match request::create_coverart_req(
                                    &app,
                                    &song_id,
                                    &resp_queue_coverart_id,
                                )
                                .await
                                {
                                    Ok(response) => {
                                        let resp = util::get_resp_data::<
                                                            soaricarus_api::callers::coverart::response::create_coverart::Response,
                                                        >(response)
                                                        .await;
                                        assert_eq!(
                                            false,
                                            resp.data.is_empty(),
                                            "Should not be empty"
                                        );
                                    }
                                    Err(err) => {
                                        assert!(false, "Error: {:?}", err);
                                    }
                                }
                            }
                            Err(err) => {
                                assert!(false, "Error: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_wipe_data_from_song_queue() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        // Send request
        match sequence_flow::queue_song_flow(&app).await {
            Ok((response, user_id)) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::metadata::response::fetch_metadata::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Data should not be empty");
                let song_q_id = resp.data[0].song_queue_id;

                match request::create_song_req(&app, &song_q_id, &user_id).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::song::response::create_metadata::Response,
                        >(response)
                        .await;
                        assert_eq!(
                            false,
                            resp.data.is_empty(),
                            "No songs found, Response {:?}",
                            resp
                        );
                        let song = &resp.data[0];
                        let song_id = song.id;
                        assert_eq!(
                            false,
                            song_id.is_nil(),
                            "Song id should not be nil {:?}",
                            song
                        );

                        let payload = serde_json::json!({
                            "song_queue_id": song_q_id
                        });

                        match app
                            .clone()
                            .oneshot(
                                request::run_post(
                                    Some(request::ReqBody::Json(payload)),
                                    soaricarus_api::callers::queue::endpoints::QUEUESONGDATAWIPE,
                                    axum::http::Method::PATCH,
                                    true,
                                )
                                .await
                                .unwrap(),
                            )
                            .await
                        {
                            Ok(response) => {
                                let resp = util::get_resp_data::<soaricarus_api::callers::queue::song::response::wipe_data_from_song_queue::Response>(response).await;
                                assert_eq!(
                                    false,
                                    resp.data.is_empty(),
                                    "Failure in wiping data from song queue {:?}",
                                    resp
                                );

                                let returned_id = &resp.data[0];
                                assert_eq!(
                                    false,
                                    returned_id.is_nil(),
                                    "Returned id should not be nil {:?}",
                                    returned_id
                                );
                                assert_eq!(
                                    *returned_id, song_q_id,
                                    "Returned id does not match sent id {:?} {:?}",
                                    returned_id, song_q_id
                                );
                            }
                            Err(err) => {
                                assert!(false, "Error: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    #[tokio::test]
    async fn test_wipe_data_from_coverart_queue() {
        let tm_pool = db_mgr::get_pool().await.unwrap();
        let db_name = db_mgr::generate_db_name().await;

        match db_mgr::create_database(&tm_pool, &db_name).await {
            Ok(_) => {
                println!("Success");
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        }

        let pool = db_mgr::connect_to_db(&db_name).await.unwrap();
        db::migrations(&pool).await;

        let app = init::app(pool).await;

        match sequence_flow::queue_song_flow(&app).await {
            Ok((response, user_id)) => {
                let resp = util::get_resp_data::<
                    soaricarus_api::callers::queue::metadata::response::fetch_metadata::Response,
                >(response)
                .await;
                assert_eq!(false, resp.data.is_empty(), "Data should not be empty");
                let song_queue_id = resp.data[0].song_queue_id;

                match request::create_song_req(&app, &song_queue_id, &user_id).await {
                    Ok(response) => {
                        let resp = util::get_resp_data::<
                            soaricarus_api::callers::song::response::create_metadata::Response,
                        >(response)
                        .await;
                        assert_eq!(
                            false,
                            resp.data.is_empty(),
                            "No songs found, Response {:?}",
                            resp
                        );
                        let song = &resp.data[0];
                        let song_id = song.id;
                        assert_eq!(
                            false,
                            song_id.is_nil(),
                            "Song id should not be nil {:?}",
                            song
                        );

                        eprintln!("Song: {:?}", song);

                        match sequence_flow::queue_coverart_flow(&app, &song_queue_id).await {
                            Ok(response) => {
                                let resp = util::get_resp_data::<
                                    soaricarus_api::callers::queue::coverart::response::fetch_coverart_no_data::Response,
                                >(response)
                                .await;
                                assert_eq!(false, resp.data.is_empty(), "Should not be empty");

                                let resp_coverart_queue_id = resp.data[0].id;

                                let payload = serde_json::json!({
                                    "coverart_queue_id": resp_coverart_queue_id
                                });

                                match app
                                    .clone()
                                    .oneshot(
                                        request::run_post(
                                            Some(request::ReqBody::Json(payload)),
                                            soaricarus_api::callers::queue::endpoints::QUEUECOVERARTDATAWIPE,
                                            axum::http::Method::PATCH,
                                            true,
                                        )
                                        .await
                                        .unwrap(),
                                    )
                                    .await
                                {
                                    Ok(response) => {
                                        let resp = util::get_resp_data::<
                                            soaricarus_api::callers::queue::coverart::response::wipe_data_from_coverart_queue::Response,
                                        >(response)
                                        .await;
                                        assert_eq!(
                                            false,
                                            resp.data.is_empty(),
                                            "Should not be empty"
                                        );
                                    }
                                    Err(err) => {
                                        assert!(false, "Error: {:?}", err);
                                    }
                                }
                            }
                            Err(err) => {
                                assert!(false, "Error: {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        assert!(false, "Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                assert!(false, "Error: {:?}", err);
            }
        };

        let _ = db_mgr::drop_database(&tm_pool, &db_name).await;
    }

    pub mod zzz_after_song_queue {
        use futures::StreamExt;
        use tower::ServiceExt;

        #[tokio::test]
        async fn test_get_songs() {
            let tm_pool = super::db_mgr::get_pool().await.unwrap();
            let db_name = super::db_mgr::generate_db_name().await;

            match super::db_mgr::create_database(&tm_pool, &db_name).await {
                Ok(_) => {
                    println!("Success");
                }
                Err(err) => {
                    assert!(false, "Error: {:?}", err);
                }
            }

            let pool = super::db_mgr::connect_to_db(&db_name).await.unwrap();
            super::db_mgr::migrations(&pool).await;

            let app = super::init::app(pool).await;

            let (id, _, _, _) = test_data::song_id().await.unwrap();

            let uri = format!("{}?id={id}", soaricarus_api::callers::endpoints::GETSONGS);

            match app
                .clone()
                .oneshot(
                    super::request::run_post(None, &uri, axum::http::Method::GET, false)
                        .await
                        .unwrap(),
                )
                .await
            {
                Ok(response) => {
                    let resp = super::util::get_resp_data::<
                        soaricarus_api::callers::song::response::get_songs::Response,
                    >(response)
                    .await;
                    assert_eq!(false, resp.data.is_empty(), "Should not be empty");

                    let song = resp.data[0].clone();
                    assert_eq!(id, song.id, "Id does not match {song:?}");
                }
                Err(err) => {
                    assert!(false, "Error: {err:?}");
                }
            }

            let _ = super::db_mgr::drop_database(&tm_pool, &db_name).await;
        }

        #[tokio::test]
        async fn test_get_coverart() {
            let tm_pool = super::db_mgr::get_pool().await.unwrap();
            let db_name = super::db_mgr::generate_db_name().await;

            match super::db_mgr::create_database(&tm_pool, &db_name).await {
                Ok(_) => {
                    println!("Success");
                }
                Err(err) => {
                    assert!(false, "Error: {:?}", err);
                }
            }

            let pool = super::db_mgr::connect_to_db(&db_name).await.unwrap();
            super::db_mgr::migrations(&pool).await;

            let app = super::init::app(pool).await;

            let id = test_data::coverart_id().await.unwrap();

            let uri = format!(
                "{}?id={id}",
                soaricarus_api::callers::endpoints::GETCOVERART
            );

            match app
                .clone()
                .oneshot(
                    super::request::run_post(None, &uri, axum::http::Method::GET, false)
                        .await
                        .unwrap(),
                )
                .await
            {
                Ok(response) => {
                    let resp = super::util::get_resp_data::<
                        soaricarus_api::callers::coverart::response::get_coverart::Response,
                    >(response)
                    .await;
                    assert_eq!(false, resp.data.is_empty(), "Should not be empty");

                    let coverart = resp.data[0].clone();
                    assert_eq!(id, coverart.id, "Id does not match {coverart:?}");
                }
                Err(err) => {
                    assert!(false, "Error: {err:?}");
                }
            }

            let _ = super::db_mgr::drop_database(&tm_pool, &db_name).await;
        }

        pub mod test_data {
            pub async fn song_id() -> Result<(uuid::Uuid, String, String, String), uuid::Error> {
                match uuid::Uuid::parse_str("44cf7940-34ff-489f-9124-d0ec90a55af9") {
                    Ok(id) => Ok((
                        id,
                        String::from("tests/I/"),
                        String::from("track01.flac"),
                        String::from("tests/I/Coverart-1.jpg"),
                    )),
                    Err(err) => Err(err),
                }
            }

            pub async fn other_song_id()
            -> Result<(uuid::Uuid, (String, String), (String, String)), uuid::Error> {
                match uuid::Uuid::parse_str("94cf7940-34ff-489f-9124-d0ec90a55af4") {
                    Ok(id) => Ok((
                        id,
                        (String::from("tests/I/"), String::from("track02.flac")),
                        (String::from("tests/I/"), String::from("Coverart-2.jpg")),
                    )),
                    Err(err) => Err(err),
                }
            }

            pub async fn coverart_id() -> Result<uuid::Uuid, uuid::Error> {
                uuid::Uuid::parse_str("996122cd-5ae9-4013-9934-60768d3006ed")
            }
        }

        #[tokio::test]
        async fn test_stream_song() {
            let tm_pool = super::db_mgr::get_pool().await.unwrap();
            let db_name = super::db_mgr::generate_db_name().await;

            match super::db_mgr::create_database(&tm_pool, &db_name).await {
                Ok(_) => {
                    println!("Success");
                }
                Err(err) => {
                    assert!(false, "Error: {:?}", err);
                }
            }

            let pool = super::db_mgr::connect_to_db(&db_name).await.unwrap();
            super::db_mgr::migrations(&pool).await;

            let app = super::init::app(pool).await;

            let (id, _, _, _) = test_data::song_id().await.unwrap();

            let my_url = soaricarus_api::callers::endpoints::STREAMSONG;
            let last = my_url.len() - 5;
            let uri = format!("{}/{id}", &my_url[0..last]);

            match app
                .clone()
                .oneshot(
                    super::request::run_post(None, &uri, axum::http::Method::GET, false)
                        .await
                        .unwrap(),
                )
                .await
            {
                Ok(response) => {
                    let e = response.into_body();
                    let mut data = e.into_data_stream();
                    while let Some(chunk) = data.next().await {
                        match chunk {
                            Ok(_data) => {}
                            Err(err) => {
                                assert!(false, "Error: {err:?}");
                            }
                        }
                    }
                }
                Err(err) => {
                    assert!(false, "Error: {err:?}");
                }
            }

            let _ = super::db_mgr::drop_database(&tm_pool, &db_name).await;
        }

        #[tokio::test]
        async fn test_download_song() {
            let tm_pool = super::db_mgr::get_pool().await.unwrap();
            let db_name = super::db_mgr::generate_db_name().await;

            match super::db_mgr::create_database(&tm_pool, &db_name).await {
                Ok(_) => {
                    println!("Success");
                }
                Err(err) => {
                    assert!(false, "Error: {:?}", err);
                }
            }

            let pool = super::db_mgr::connect_to_db(&db_name).await.unwrap();
            super::db_mgr::migrations(&pool).await;

            let app = super::init::app(pool).await;

            let (id, _, _, _) = test_data::song_id().await.unwrap();

            let uri = super::util::format_url_with_value(
                soaricarus_api::callers::endpoints::DOWNLOADSONG,
                &id,
            )
            .await;

            match app
                .clone()
                .oneshot(
                    super::request::run_post(None, &uri, axum::http::Method::GET, false)
                        .await
                        .unwrap(),
                )
                .await
            {
                Ok(response) => {
                    let e = response.into_body();
                    let mut data = e.into_data_stream();
                    while let Some(chunk) = data.next().await {
                        match chunk {
                            Ok(_data) => {}
                            Err(err) => {
                                assert!(false, "Error: {err:?}");
                            }
                        }
                    }
                }
                Err(err) => {
                    assert!(false, "Error: {err:?}");
                }
            }

            let _ = super::db_mgr::drop_database(&tm_pool, &db_name).await;
        }

        #[tokio::test]
        async fn test_download_coverart() {
            let tm_pool = super::db_mgr::get_pool().await.unwrap();
            let db_name = super::db_mgr::generate_db_name().await;

            match super::db_mgr::create_database(&tm_pool, &db_name).await {
                Ok(_) => {
                    println!("Success");
                }
                Err(err) => {
                    assert!(false, "Error: {:?}", err);
                }
            }

            let pool = super::db_mgr::connect_to_db(&db_name).await.unwrap();
            super::db_mgr::migrations(&pool).await;

            let app = super::init::app(pool).await;

            let id = test_data::coverart_id().await.unwrap();

            let uri = super::util::format_url_with_value(
                soaricarus_api::callers::endpoints::DOWNLOADCOVERART,
                &id,
            )
            .await;

            match app
                .clone()
                .oneshot(
                    super::request::run_post(None, &uri, axum::http::Method::GET, false)
                        .await
                        .unwrap(),
                )
                .await
            {
                Ok(response) => {
                    let e = response.into_body();
                    let mut data = e.into_data_stream();
                    while let Some(chunk) = data.next().await {
                        match chunk {
                            Ok(_data) => {}
                            Err(err) => {
                                assert!(false, "Error: {err:?}");
                            }
                        }
                    }
                }
                Err(err) => {
                    assert!(false, "Error: {err:?}");
                }
            }

            let _ = super::db_mgr::drop_database(&tm_pool, &db_name).await;
        }

        async fn get_test_data(
            song_directory: &String,
            song_filename: &String,
            coverart_directory: &String,
            coverart_filename: &String,
        ) -> Result<(Vec<u8>, Vec<u8>), std::io::Error> {
            let song = simodels::song::Song {
                directory: song_directory.clone(),
                filename: song_filename.clone(),
                ..Default::default()
            };

            let coverart = simodels::coverart::CoverArt {
                directory: coverart_directory.clone(),
                filename: coverart_filename.clone(),
                ..Default::default()
            };

            match simodels::song::io::to_data(&song) {
                Ok(song_data) => match simodels::coverart::io::to_data(&coverart) {
                    Ok(coverart_data) => Ok((song_data, coverart_data)),
                    Err(err) => Err(err),
                },
                Err(err) => Err(err),
            }
        }

        async fn save_test_again(
            song_directory: &String,
            song_filename: &String,
            song_data: Vec<u8>,
            coverart_directory: &String,
            coverart_filename: &String,
            coverart_data: Vec<u8>,
        ) -> Result<(), std::io::Error> {
            let song = simodels::song::Song {
                directory: song_directory.clone(),
                filename: song_filename.clone(),
                data: song_data,
                ..Default::default()
            };

            let coverart = simodels::coverart::CoverArt {
                directory: coverart_directory.clone(),
                filename: coverart_filename.clone(),
                data: coverart_data,
                ..Default::default()
            };

            match song.save_to_filesystem() {
                Ok(_) => match coverart.save_to_filesystem() {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err),
                },
                Err(err) => Err(err),
            }
        }

        #[tokio::test]
        async fn test_last_delete_song() {
            let tm_pool = super::db_mgr::get_pool().await.unwrap();
            let db_name = super::db_mgr::generate_db_name().await;

            match super::db_mgr::create_database(&tm_pool, &db_name).await {
                Ok(_) => {
                    println!("Success");
                }
                Err(err) => {
                    assert!(false, "Error: {:?}", err);
                }
            }

            let pool = super::db_mgr::connect_to_db(&db_name).await.unwrap();
            super::db_mgr::migrations(&pool).await;

            let app = super::init::app(pool).await;

            let (id, (song_directory, song_filename), (cover_directory, cover_filename)) =
                test_data::other_song_id().await.unwrap();
            let (song_data, coverart_data) = get_test_data(
                &song_directory,
                &song_filename,
                &cover_directory,
                &cover_filename,
            )
            .await
            .unwrap();

            let uri = super::util::format_url_with_value(
                soaricarus_api::callers::endpoints::DELETESONG,
                &id,
            )
            .await;

            match app
                .clone()
                .oneshot(
                    super::request::run_post(None, &uri, axum::http::Method::DELETE, false)
                        .await
                        .unwrap(),
                )
                .await
            {
                Ok(response) => {
                    let resp = super::util::get_resp_data::<
                        soaricarus_api::callers::song::response::delete_song::Response,
                    >(response)
                    .await;
                    assert_eq!(false, resp.data.is_empty(), "Response has no data");

                    let song_and_coverart = &resp.data[0];
                    assert_eq!(
                        id, song_and_coverart.song.id,
                        "Song Ids do not match {id:?} {:?}",
                        song_and_coverart.song.id
                    );

                    match save_test_again(
                        &song_directory,
                        &song_filename,
                        song_data,
                        &cover_directory,
                        &cover_filename,
                        coverart_data,
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            assert!(false, "Error: {err:?}");
                        }
                    }
                }
                Err(err) => {
                    assert!(false, "Error: {err:?}");
                }
            }

            let _ = super::db_mgr::drop_database(&tm_pool, &db_name).await;
        }

        #[tokio::test]
        async fn test_get_all_songs() {
            let tm_pool = super::db_mgr::get_pool().await.unwrap();
            let db_name = super::db_mgr::generate_db_name().await;

            match super::db_mgr::create_database(&tm_pool, &db_name).await {
                Ok(_) => {
                    println!("Success");
                }
                Err(err) => {
                    assert!(false, "Error: {:?}", err);
                }
            }

            let pool = super::db_mgr::connect_to_db(&db_name).await.unwrap();
            super::db_mgr::migrations(&pool).await;

            let app = super::init::app(pool).await;

            match app
                .clone()
                .oneshot(
                    super::request::run_post(
                        None,
                        soaricarus_api::callers::endpoints::GETALLSONGS,
                        axum::http::Method::GET,
                        false,
                    )
                    .await
                    .unwrap(),
                )
                .await
            {
                Ok(response) => {
                    let resp = super::util::get_resp_data::<
                        soaricarus_api::callers::song::response::get_songs::Response,
                    >(response)
                    .await;
                    assert_eq!(false, resp.data.is_empty(), "Should not be empty");

                    let songs = &resp.data;
                    assert_eq!(
                        2,
                        songs.len(),
                        "Returned song count does not match. Returned song count {:?} song count {}",
                        songs.len(),
                        2
                    );
                }
                Err(err) => {
                    assert!(false, "Error: {err:?}");
                }
            }

            let _ = super::db_mgr::drop_database(&tm_pool, &db_name).await;
        }
    }
}
