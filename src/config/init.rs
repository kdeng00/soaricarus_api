use std::time::Duration;

use axum::routing::{delete, get, patch, post};
use tower_http::timeout::TimeoutLayer;
use utoipa::OpenApi;

use crate::callers::coverart as coverart_caller;
use crate::callers::queue::coverart as coverart_queue_callers;
use crate::callers::queue::metadata as metadata_queue_caller;
use crate::callers::queue::song as song_queue_callers;
use crate::callers::song as song_caller;
use coverart_caller::endpoint as coverart_endpoints;
use coverart_caller::response as coverart_responses;
use coverart_queue_callers::endpoint as coverart_queue_endpoints;
use coverart_queue_callers::response as coverart_queue_responses;
use metadata_queue_caller::endpoint as metadata_queue_endpoints;
use metadata_queue_caller::response as metadata_queue_responses;
use song_caller::endpoint as song_endpoints;
use song_caller::response as song_responses;
use song_queue_callers::endpoint as song_queue_endpoints;
use song_queue_callers::response as song_queue_responses;

mod cors {
    pub async fn configure_cors() -> tower_http::cors::CorsLayer {
        let cors = tower_http::cors::CorsLayer::new()
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ]) // Specify allowed methods:cite[2]
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
            ]) // Specify allowed headers:cite[2]
            .allow_credentials(true) // If you need to send cookies or authentication headers:cite[2]
            .max_age(std::time::Duration::from_secs(3600)); // Cache the preflight response for 1 hour:cite[2]

        // Dynamically set the allowed origin based on the environment
        match std::env::var(icarus_envy::keys::APP_ENV).as_deref() {
            Ok("production") => {
                // In production, allow only your specific, trusted origins
                let allowed_origins_env = icarus_envy::environment::get_allowed_origins();
                match icarus_envy::utility::delimitize(&allowed_origins_env) {
                    Ok(alwd) => {
                        let allowed_origins: Vec<axum::http::HeaderValue> = alwd
                            .into_iter()
                            .map(|a| a.parse::<axum::http::HeaderValue>().unwrap())
                            .collect();
                        cors.allow_origin(allowed_origins)
                    }
                    Err(err) => {
                        eprintln!("Error getting allowed origins: Error: {err:?}");
                        std::process::exit(-1);
                    }
                }
            }
            _ => {
                // Development (default): Allow localhost origins
                cors.allow_origin(vec![
                    "http://localhost:8000".parse().unwrap(),
                    "http://127.0.0.1:8000".parse().unwrap(),
                    "http://localhost:8001".parse().unwrap(),
                    "http://127.0.0.1:8001".parse().unwrap(),
                    "http://localhost:4200".parse().unwrap(),
                    "http://127.0.0.1:4200".parse().unwrap(),
                ])
            }
        }
    }
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(song_queue_endpoints::queue_song, song_queue_endpoints::link_user_id, song_queue_endpoints::fetch_queue_song, song_queue_endpoints::download_queued_song,
        song_queue_endpoints::update_song_queue_status, song_queue_endpoints::update_song_queue, song_endpoints::create_metadata, song_queue_endpoints::wipe_data_from_song_queue, song_endpoints::get_songs, song_endpoints::get_all_songs, song_endpoints::stream_song, song_endpoints::download_song,
        song_endpoints::delete_song, coverart_queue_endpoints::queue, coverart_queue_endpoints::link, coverart_queue_endpoints::fetch_coverart_no_data,
        coverart_queue_endpoints::fetch_coverart_with_data, coverart_endpoints::create_coverart, coverart_queue_endpoints::wipe_data_from_coverart_queue,
        coverart_endpoints::get_coverart, coverart_endpoints::download_coverart,
        metadata_queue_endpoints::queue_metadata, metadata_queue_endpoints::fetch_metadata),
    components(schemas(song_queue_callers::response::song_queue::Response, song_queue_callers::response::link_user_id::Response, song_queue_callers::response::fetch_queue_song::Response,
            song_queue_responses::update_status::Response, song_queue_callers::response::update_song_queue::Response, song_responses::create_metadata::Response,
            song_queue_callers::response::wipe_data_from_song_queue::Response, song_responses::get_songs::Response, song_responses::delete_song::Response,
            coverart_queue_responses::queue::Response, coverart_queue_responses::link::Response, coverart_queue_responses::fetch_coverart_no_data::Response,
            coverart_queue_responses::fetch_coverart_with_data::Response, coverart_responses::create_coverart::Response,
            coverart_queue_responses::wipe_data_from_coverart_queue::Response, coverart_responses::get_coverart::Response,
            metadata_queue_responses::queue_metadata::Response, metadata_queue_responses::fetch_metadata::Response)),
    tags(
        (name = "Icarus API", description = "Web API to manage music")
    )
)]
struct ApiDoc;

pub async fn routes() -> axum::Router {
    axum::Router::new()
        .route(crate::callers::endpoints::ROOT, get(crate::callers::root))
        .route(
            crate::callers::queue::endpoints::QUEUESONG,
            post(crate::callers::queue::song::endpoint::queue_song).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUESONG,
            patch(crate::callers::queue::song::endpoint::update_song_queue_status).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUESONGLINKUSERID,
            patch(crate::callers::queue::song::endpoint::link_user_id).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUESONGDATA,
            get(crate::callers::queue::song::endpoint::download_queued_song).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::NEXTQUEUESONG,
            get(crate::callers::queue::song::endpoint::fetch_queue_song).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUESONGUPDATE,
            patch(crate::callers::queue::song::endpoint::update_song_queue).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUESONGDATAWIPE,
            patch(crate::callers::queue::song::endpoint::wipe_data_from_song_queue).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUEMETADATA,
            post(crate::callers::queue::metadata::endpoint::queue_metadata).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUEMETADATA,
            get(crate::callers::queue::metadata::endpoint::fetch_metadata).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUECOVERART,
            post(crate::callers::queue::coverart::endpoint::queue).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUECOVERARTDATA,
            get(crate::callers::queue::coverart::endpoint::fetch_coverart_with_data).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUECOVERART,
            get(crate::callers::queue::coverart::endpoint::fetch_coverart_no_data).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUECOVERARTLINK,
            patch(crate::callers::queue::coverart::endpoint::link).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::queue::endpoints::QUEUECOVERARTDATAWIPE,
            patch(crate::callers::queue::coverart::endpoint::wipe_data_from_coverart_queue)
                .route_layer(axum::middleware::from_fn(
                    crate::auth::auth::<axum::body::Body>,
                )),
        )
        .route(
            crate::callers::endpoints::CREATESONG,
            post(crate::callers::song::endpoint::create_metadata).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::endpoints::CREATECOVERART,
            post(crate::callers::coverart::endpoint::create_coverart).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::endpoints::GETSONGS,
            get(crate::callers::song::endpoint::get_songs).route_layer(axum::middleware::from_fn(
                crate::auth::auth::<axum::body::Body>,
            )),
        )
        .route(
            crate::callers::endpoints::GETCOVERART,
            get(crate::callers::coverart::endpoint::get_coverart).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::endpoints::DOWNLOADCOVERART,
            get(crate::callers::coverart::endpoint::download_coverart).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::endpoints::STREAMSONG,
            get(crate::callers::song::endpoint::stream_song).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::endpoints::DOWNLOADSONG,
            get(crate::callers::song::endpoint::download_song).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::endpoints::DELETESONG,
            delete(crate::callers::song::endpoint::delete_song).route_layer(
                axum::middleware::from_fn(crate::auth::auth::<axum::body::Body>),
            ),
        )
        .route(
            crate::callers::endpoints::GETALLSONGS,
            get(crate::callers::song::endpoint::get_all_songs),
        )
        .layer(cors::configure_cors().await)
}

pub async fn app() -> axum::Router {
    let pool = crate::db::create_pool()
        .await
        .expect("Failed to create pool");

    crate::db::migrations(&pool).await;

    let cors = cors::configure_cors().await;

    routes()
        .await
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .layer(axum::Extension(pool))
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::OK,
            Duration::from_secs(300),
        ))
        .layer(cors)
}
