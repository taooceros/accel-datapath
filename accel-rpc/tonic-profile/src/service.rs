use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use hdrhistogram::Histogram;
use prost::Message;
use tokio::sync::{Mutex, Notify};
use tonic::codec::CompressionEncoding;
use tonic::{Request, Response, Status};

use crate::cli::{Args, CompressionMode};
use crate::profile::echo_reply::Body as EchoReplyBody;
use crate::profile::echo_request::Body as EchoRequestBody;
use crate::profile::profile_server::{Profile, ProfileServer};
use crate::profile::{EchoReply, EchoRequest};
use crate::workload::{proto_shape_from_message, proto_shape_from_wire, ProtoPools};
use crate::{BoxError, Metrics};

#[derive(Default)]
pub(crate) struct SharedState {
    pub(crate) shutdown: Arc<Notify>,
}

pub(crate) struct ServerMetrics {
    pub(crate) requests_completed: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    latency_hist: Mutex<Histogram<u64>>,
    started_at: Instant,
}

impl ServerMetrics {
    pub(crate) fn new() -> Result<Self, BoxError> {
        Ok(Self {
            requests_completed: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            latency_hist: Mutex::new(Histogram::<u64>::new(3)?),
            started_at: Instant::now(),
        })
    }

    pub(crate) async fn record_request(
        &self,
        request_bytes: usize,
        response_bytes: usize,
        latency_us: u64,
    ) {
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        self.bytes_received
            .fetch_add(request_bytes as u64, Ordering::Relaxed);
        self.bytes_sent
            .fetch_add(response_bytes as u64, Ordering::Relaxed);
        let mut hist = self.latency_hist.lock().await;
        let _ = hist.record(latency_us.max(1));
    }

    pub(crate) async fn snapshot(&self) -> Metrics {
        let duration = self.started_at.elapsed();
        let duration_s = duration.as_secs_f64().max(1e-9);
        let requests_completed = self.requests_completed.load(Ordering::Relaxed);
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);
        let hist = self.latency_hist.lock().await;

        Metrics {
            requests_completed,
            bytes_sent,
            bytes_received,
            duration_ms: duration.as_secs_f64() * 1000.0,
            throughput_rps: requests_completed as f64 / duration_s,
            throughput_mib_s: bytes_sent as f64 / duration_s / (1024.0 * 1024.0),
            latency_us_p50: if requests_completed == 0 {
                0
            } else {
                hist.value_at_quantile(0.50)
            },
            latency_us_p95: if requests_completed == 0 {
                0
            } else {
                hist.value_at_quantile(0.95)
            },
            latency_us_p99: if requests_completed == 0 {
                0
            } else {
                hist.value_at_quantile(0.99)
            },
            latency_us_max: if requests_completed == 0 { 0 } else { hist.max() },
        }
    }
}

pub(crate) struct EchoSvc {
    compression: CompressionMode,
    proto_pools: Arc<ProtoPools>,
    next_proto_index: AtomicU64,
    shutdown: Arc<Notify>,
    shutdown_after_requests: Option<u64>,
    server_metrics: Arc<ServerMetrics>,
}

pub(crate) fn build_profile_server(
    args: &Args,
    shared: &Arc<SharedState>,
    server_metrics: &Arc<ServerMetrics>,
) -> ProfileServer<EchoSvc> {
    let mut svc = ProfileServer::new(EchoSvc {
        compression: args.compression,
        proto_pools: Arc::new(ProtoPools::new()),
        next_proto_index: AtomicU64::new(0),
        shutdown: shared.shutdown.clone(),
        shutdown_after_requests: args.shutdown_after_requests,
        server_metrics: server_metrics.clone(),
    });
    if args.compression == CompressionMode::On {
        svc = svc
            .accept_compressed(CompressionEncoding::Gzip)
            .send_compressed(CompressionEncoding::Gzip);
    }
    svc
}

#[tonic::async_trait]
impl Profile for EchoSvc {
    async fn unary_echo(
        &self,
        request: Request<EchoRequest>,
    ) -> Result<Response<EchoReply>, Status> {
        let started = Instant::now();
        let request = request.into_inner();
        let request_bytes = request.encoded_len();
        let body = request
            .body
            .ok_or_else(|| Status::invalid_argument("missing request body"))?;

        let reply_body = match body {
            EchoRequestBody::Payload(payload) => EchoReplyBody::Payload(payload),
            EchoRequestBody::ProtoPayload(proto_payload) => {
                let request_shape = proto_shape_from_message(&proto_payload)
                    .ok_or_else(|| Status::invalid_argument("missing proto shape body"))?;
                let response_shape = proto_shape_from_wire(request.requested_response_shape)?
                    .unwrap_or(request_shape);
                let response = self.proto_pools.pick(
                    response_shape,
                    self.next_proto_index.fetch_add(1, Ordering::Relaxed),
                );
                EchoReplyBody::ProtoPayload(response)
            }
        };

        let response_message = EchoReply {
            body: Some(reply_body),
        };
        let response_bytes = response_message.encoded_len();
        let mut response = Response::new(response_message);
        if self.compression == CompressionMode::On {
            response
                .metadata_mut()
                .insert("x-tonic-profile", "gzip".parse().unwrap());
        }
        self.server_metrics
            .record_request(
                request_bytes,
                response_bytes,
                started.elapsed().as_micros() as u64,
            )
            .await;
        if let Some(target) = self.shutdown_after_requests {
            let completed = self.server_metrics.requests_completed.load(Ordering::Relaxed);
            if completed >= target {
                self.shutdown.notify_waiters();
            }
        }
        Ok(response)
    }
}
