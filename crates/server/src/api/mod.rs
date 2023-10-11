use crate::{
    extractor::{interfaces::Interface, Extractor},
    policy::{content::ContentPolicy, record::RecordPolicy},
    services::CoreService,
};
use axum::{body::Body, http::Request, Router};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{Level, Span};
use url::Url;
use wasm_metadata::RegistryMetadata;

pub mod v1;

#[cfg(feature = "debug")]
pub mod debug;

/// Creates the router for the API.
#[allow(clippy::too_many_arguments)]
pub fn create_router(
    content_base_url: Url,
    core: CoreService,
    temp_dir: PathBuf,
    files_dir: PathBuf,
    metadata_extractor: Option<Arc<dyn Extractor<RegistryMetadata>>>,
    interface_extractor: Option<Arc<dyn Extractor<Vec<Interface>>>>,
    content_policy: Option<Arc<dyn ContentPolicy>>,
    record_policy: Option<Arc<dyn RecordPolicy>>,
) -> Router {
    let router = Router::new();
    #[cfg(feature = "debug")]
    let router = router.nest("/debug", debug::Config::new(core.clone()).into_router());
    router
        .nest(
            "/v1",
            v1::create_router(
                content_base_url,
                core,
                temp_dir,
                files_dir.clone(),
                metadata_extractor,
                interface_extractor,
                content_policy,
                record_policy,
            ),
        )
        .nest_service("/content", ServeDir::new(files_dir))
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().include_headers(true))
                        .on_request(|request: &Request<Body>, _span: &Span| {
                            tracing::info!("starting {} {}", request.method(), request.uri().path())
                        })
                        .on_response(
                            DefaultOnResponse::new()
                                .level(Level::INFO)
                                .latency_unit(LatencyUnit::Micros),
                        ),
                )
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                        .allow_headers([
                            axum::http::header::CONTENT_TYPE,
                            axum::http::header::ACCEPT,
                        ]),
                ),
        )
}
