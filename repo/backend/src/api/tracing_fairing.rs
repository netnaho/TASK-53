use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};
use uuid::Uuid;

use crate::application::metrics_service::MetricsService;

pub struct TracingFairing;

#[rocket::async_trait]
impl Fairing for TracingFairing {
    fn info(&self) -> Info {
        Info {
            name: "Request Tracing & Metrics",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut rocket::Data<'_>) {
        let trace_id = request
            .headers()
            .get_one("X-Trace-Id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        tracing::info!(
            trace_id = %trace_id,
            method = %request.method(),
            uri = %request.uri(),
            "request.start"
        );

        request.local_cache(|| trace_id);
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let trace_id = request.local_cache(|| Uuid::new_v4().to_string());
        response.set_header(Header::new("X-Trace-Id", trace_id.clone()));

        let status = response.status();
        let is_server_error = status.code >= 500;

        // Record into the metrics service if it's available in Rocket state
        if let Some(metrics) = request.rocket().state::<MetricsService>() {
            metrics.record(is_server_error);
        }

        tracing::info!(
            trace_id = %trace_id,
            status = status.code,
            is_error = is_server_error,
            "request.complete"
        );
    }
}
