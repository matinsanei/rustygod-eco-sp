//! Observability: per-RPC metrics (Prometheus) + structured trace spans.
//!
//! Every gRPC call flows through [`MetricsLayer`], which records:
//! - `grpc_requests_total{service,method}` — counter
//! - `grpc_request_duration_seconds{service,method}` — histogram
//!
//! `service`/`method` come from the gRPC path (`/pkg.Service/Method`).
//! Honest limit: tonic encodes the RPC status in response *trailers*, so a
//! header-only tower layer cannot label per-RPC codes without buffering
//! bodies — failed calls surface here as latency + server error logs, and
//! business errors are already explicit in every response's `errors` field.
//! Scrape via the Prometheus exporter (`RUSTYGOD_METRICS_ADDR`, default
//! `127.0.0.1:9000`). Spans come from
//! `tower_http::trace::TraceLayer::new_for_grpc` (wired in `main.rs`).

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use tower::{Layer, Service};

#[derive(Clone, Default)]
pub struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService { inner }
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
}

/// Split `/rustygod.draft.DraftOrderService/CreateDraftOrder`
/// into `("rustygod.draft.DraftOrderService", "CreateDraftOrder")`.
fn split_grpc_path(path: &str) -> (&str, &str) {
    let trimmed = path.strip_prefix('/').unwrap_or(path);
    match trimmed.rfind('/') {
        Some(i) => (&trimmed[..i], &trimmed[i + 1..]),
        None => ("unknown", trimmed),
    }
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for MetricsService<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = MetricsFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<ReqBody>) -> Self::Future {
        let path = req.uri().path().to_string();
        let (service, method) = split_grpc_path(&path);
        MetricsFuture {
            inner: Box::pin(self.inner.call(req)),
            service: service.to_string(),
            method: method.to_string(),
            start: Instant::now(),
        }
    }
}

pub struct MetricsFuture<F> {
    inner: Pin<Box<F>>,
    service: String,
    method: String,
    start: Instant,
}

impl<F, ResBody, E> Future for MetricsFuture<F>
where
    F: Future<Output = Result<http::Response<ResBody>, E>>,
{
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let out = std::task::ready!(Pin::new(&mut self.inner).poll(cx));
        let elapsed = self.start.elapsed().as_secs_f64();
        metrics::counter!(
            "grpc_requests_total",
            "service" => self.service.clone(),
            "method" => self.method.clone(),
        )
        .increment(1);
        metrics::histogram!(
            "grpc_request_duration_seconds",
            "service" => self.service.clone(),
            "method" => self.method.clone(),
        )
        .record(elapsed);
        Poll::Ready(out)
    }
}

#[cfg(test)]
mod tests {
    use super::split_grpc_path;

    #[test]
    fn grpc_path_splits() {
        assert_eq!(
            split_grpc_path("/rustygod.draft.DraftOrderService/CreateDraftOrder"),
            ("rustygod.draft.DraftOrderService", "CreateDraftOrder")
        );
        assert_eq!(split_grpc_path("/health"), ("unknown", "health"));
    }
}
