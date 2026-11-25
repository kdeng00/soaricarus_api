use std::time::Duration;

use axum::body::Body as AxumBody;
use axum::routing::{delete, get, patch, post};
use tower_http::timeout::TimeoutLayer;
use utoipa::OpenApi;

use crate::auth::auth;
use crate::callers;
use crate::db;
use callers::coverart as coverart_caller;
use callers::endpoints;
use callers::playback::queue as playback_caller;
use callers::queue::coverart as coverart_queue_callers;
use callers::queue::metadata as metadata_queue_caller;
use callers::queue::song as song_queue_callers;
use callers::song as song_caller;

use coverart_caller::endpoint as coverart_endpoints;
use coverart_caller::response as coverart_responses;
use coverart_queue_callers::endpoint as coverart_queue_endpoints;
use coverart_queue_callers::response as coverart_queue_responses;
use metadata_queue_caller::endpoint as metadata_queue_endpoints;
use metadata_queue_caller::response as metadata_queue_responses;
use playback_caller::endpoint as playback_endpoints;
use playback_caller::response as playback_responses;
use song_caller::endpoint as song_endpoints;
use song_caller::response as song_responses;
use song_queue_callers::endpoint as song_queue_endpoints;
use song_queue_callers::response as song_queue_responses;

mod cors {
    pub async fn configure_cors() -> tower_http::cors::CorsLayer {
        let cors = tower_http::cors::CorsLayer::new()
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::PATCH,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
            ])
            .allow_credentials(true)
            .max_age(std::time::Duration::from_secs(3600)); // Cache the preflight response for 1 hour:cite[2]

        match std::env::var(sienvy::keys::APP_ENV).as_deref() {
            Ok("production") => {
                let allowed_origins_env = sienvy::environment::get_allowed_origins();
                match sienvy::utility::delimitize(&allowed_origins_env) {
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
            _ => cors.allow_origin(vec![
                "http://localhost:8000".parse().unwrap(),
                "http://127.0.0.1:8000".parse().unwrap(),
                "http://localhost:8001".parse().unwrap(),
                "http://127.0.0.1:8001".parse().unwrap(),
                "http://localhost:4200".parse().unwrap(),
                "http://127.0.0.1:4200".parse().unwrap(),
            ]),
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
        metadata_queue_endpoints::queue_metadata, metadata_queue_endpoints::fetch_metadata, playback_endpoints::create_playback_queue,
    playback_endpoints::get_playback_queue,
    playback_endpoints::add_to_playback_queue, playback_endpoints::clear_playback_queue),
    components(schemas(song_queue_callers::response::song_queue::Response, song_queue_callers::response::link_user_id::Response, song_queue_callers::response::fetch_queue_song::Response,
            song_queue_responses::update_status::Response, song_queue_callers::response::update_song_queue::Response, song_responses::create_metadata::Response,
            song_queue_callers::response::wipe_data_from_song_queue::Response, song_responses::get_songs::Response, song_responses::delete_song::Response,
            coverart_queue_responses::queue::Response, coverart_queue_responses::link::Response, coverart_queue_responses::fetch_coverart_no_data::Response,
            coverart_queue_responses::fetch_coverart_with_data::Response, coverart_responses::create_coverart::Response,
            coverart_queue_responses::wipe_data_from_coverart_queue::Response, coverart_responses::get_coverart::Response,
            metadata_queue_responses::queue_metadata::Response, metadata_queue_responses::fetch_metadata::Response, playback_responses::create_playback_queue::CreatePlaybackQueueResponse,
        playback_responses::GetPlaybackQueueResponse, playback_responses::AddPlaybackQueueItemResponse,
        playback_responses::ClearPlaybackQueueResponse
    )),
    tags(
        (name = "soaricarus API", description = "Web API to manage music")
    )
)]
struct ApiDoc;

pub async fn routes() -> axum::Router {
    axum::Router::new()
        .route(endpoints::ROOT, get(callers::root))
        .route(
            callers::queue::endpoints::QUEUESONG,
            post(callers::queue::song::endpoint::queue_song)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUESONG,
            patch(callers::queue::song::endpoint::update_song_queue_status)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUESONGLINKUSERID,
            patch(callers::queue::song::endpoint::link_user_id)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUESONGDATA,
            get(callers::queue::song::endpoint::download_queued_song)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::NEXTQUEUESONG,
            get(callers::queue::song::endpoint::fetch_queue_song)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUESONGUPDATE,
            patch(callers::queue::song::endpoint::update_song_queue)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUESONGDATAWIPE,
            patch(callers::queue::song::endpoint::wipe_data_from_song_queue)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUEMETADATA,
            post(callers::queue::metadata::endpoint::queue_metadata)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUEMETADATA,
            get(callers::queue::metadata::endpoint::fetch_metadata)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUECOVERART,
            post(callers::queue::coverart::endpoint::queue)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUECOVERARTDATA,
            get(callers::queue::coverart::endpoint::fetch_coverart_with_data)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUECOVERART,
            get(callers::queue::coverart::endpoint::fetch_coverart_no_data)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUECOVERARTLINK,
            patch(callers::queue::coverart::endpoint::link)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            callers::queue::endpoints::QUEUECOVERARTDATAWIPE,
            patch(callers::queue::coverart::endpoint::wipe_data_from_coverart_queue)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::CREATESONG,
            post(callers::song::endpoint::create_metadata)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::CREATECOVERART,
            post(callers::coverart::endpoint::create_coverart)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::GETSONGS,
            get(callers::song::endpoint::get_songs)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::GETCOVERART,
            get(callers::coverart::endpoint::get_coverart)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::DOWNLOADCOVERART,
            get(callers::coverart::endpoint::download_coverart)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::STREAMSONG,
            get(callers::song::endpoint::stream_song)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::DOWNLOADSONG,
            get(callers::song::endpoint::download_song)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::DELETESONG,
            delete(callers::song::endpoint::delete_song)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::GETALLSONGS,
            get(callers::song::endpoint::get_all_songs)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::CREATEPLAYBACKQUEUE,
            post(playback_endpoints::create_playback_queue)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::GETPLAYBACKQUEUE,
            get(playback_endpoints::get_playback_queue)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::ADDPLAYBACKQUEUE,
            post(playback_endpoints::add_to_playback_queue)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::GETPLAYBACKQUEUEITEM,
            get(playback_endpoints::get_playback_queue_item)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .route(
            endpoints::CLEARPLAYBACKQUEUE,
            post(playback_endpoints::clear_playback_queue)
                .route_layer(axum::middleware::from_fn(auth::<AxumBody>)),
        )
        .layer(cors::configure_cors().await)
}

pub async fn app() -> axum::Router {
    let pool = db::create_pool().await.expect("Failed to create pool");

    db::migrations(&pool).await;

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
