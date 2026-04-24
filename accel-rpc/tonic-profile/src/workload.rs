use prost::Message;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use serde::Serialize;
use tonic::Status;

use crate::cli::{Args, PayloadKind, ProtoShape, ResponseShape, RpcMode};
use crate::profile::echo_reply::Body as EchoReplyBody;
use crate::profile::echo_request::Body as EchoRequestBody;
use crate::profile::{
    self, CompactProfileShape, EchoReply, EchoRequest, FleetResponseEntry,
    FleetResponseHeavyShape, FleetSmallShape, FleetStringHeavyShape, FleetStringLeaf, ShapeKind,
};
use crate::BoxError;

const REQUEST_POOL_LEN: usize = 4;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SelectionPolicy {
    EchoPayload,
    SameAsRequest,
    ExplicitResponse,
}

#[derive(Clone)]
pub(crate) struct PreparedRequest {
    body: PreparedRequestBody,
    pub(crate) expected_response_shape: Option<ProtoShape>,
    pub(crate) request_serialized_size: usize,
    pub(crate) response_serialized_size: usize,
}

#[derive(Clone)]
enum PreparedRequestBody {
    Bytes(Vec<u8>),
    Proto(CompactProfileShape),
}

#[derive(Clone)]
pub(crate) struct WorkloadDescriptor {
    pub(crate) workload_label: String,
    pub(crate) selection_policy: SelectionPolicy,
    pub(crate) request_shape: Option<ProtoShape>,
    pub(crate) response_shape: Option<ProtoShape>,
    pub(crate) payload_size: Option<usize>,
    pub(crate) payload_kind: Option<PayloadKind>,
    pub(crate) request_serialized_size: usize,
    pub(crate) response_serialized_size: usize,
}

pub(crate) struct ProtoPools {
    fleet_small: Vec<CompactProfileShape>,
    fleet_string_heavy: Vec<CompactProfileShape>,
    fleet_response_heavy: Vec<CompactProfileShape>,
}

impl ProtoPools {
    pub(crate) fn new() -> Self {
        let fleet_small = (0..REQUEST_POOL_LEN)
            .map(|seed| build_proto_message(ProtoShape::FleetSmall, seed as u64))
            .collect();
        let fleet_string_heavy = (0..REQUEST_POOL_LEN)
            .map(|seed| build_proto_message(ProtoShape::FleetStringHeavy, seed as u64))
            .collect();
        let fleet_response_heavy = (0..REQUEST_POOL_LEN)
            .map(|seed| build_proto_message(ProtoShape::FleetResponseHeavy, seed as u64))
            .collect();
        Self {
            fleet_small,
            fleet_string_heavy,
            fleet_response_heavy,
        }
    }

    pub(crate) fn pick(&self, shape: ProtoShape, index: u64) -> CompactProfileShape {
        let pool = match shape {
            ProtoShape::FleetSmall => &self.fleet_small,
            ProtoShape::FleetStringHeavy => &self.fleet_string_heavy,
            ProtoShape::FleetResponseHeavy => &self.fleet_response_heavy,
        };
        pool[index as usize % pool.len()].clone()
    }
}

pub(crate) fn workload_probe(args: &Args) -> WorkloadDescriptor {
    let pool = build_request_pool(args, 0);
    let first = pool.first().expect("workload pool should not be empty");
    let request_shape = match args.rpc {
        RpcMode::UnaryBytes => None,
        RpcMode::UnaryProtoShape => args.proto_shape,
    };
    let response_shape = match args.rpc {
        RpcMode::UnaryBytes => None,
        RpcMode::UnaryProtoShape => Some(resolve_response_shape(
            request_shape.expect("proto shape required for unary-proto-shape"),
            args.response_shape,
        )),
    };
    let selection_policy = match args.rpc {
        RpcMode::UnaryBytes => SelectionPolicy::EchoPayload,
        RpcMode::UnaryProtoShape => match args.response_shape {
            ResponseShape::Same => SelectionPolicy::SameAsRequest,
            _ => SelectionPolicy::ExplicitResponse,
        },
    };
    let workload_label = match args.rpc {
        RpcMode::UnaryBytes => format!(
            "ordinary/unary-bytes/{}-{}",
            serialize_payload_kind(args.payload_kind),
            args.payload_size
        ),
        RpcMode::UnaryProtoShape => format!(
            "ordinary/unary-proto-shape/{}-to-{}",
            serialize_proto_shape(request_shape.expect("proto shape")),
            serialize_proto_shape(response_shape.expect("response shape"))
        ),
    };

    WorkloadDescriptor {
        workload_label,
        selection_policy,
        request_shape,
        response_shape,
        payload_size: match args.rpc {
            RpcMode::UnaryBytes => Some(args.payload_size),
            RpcMode::UnaryProtoShape => None,
        },
        payload_kind: match args.rpc {
            RpcMode::UnaryBytes => Some(args.payload_kind),
            RpcMode::UnaryProtoShape => None,
        },
        request_serialized_size: first.request_serialized_size,
        response_serialized_size: first.response_serialized_size,
    }
}

pub(crate) fn build_request_pool(args: &Args, worker_seed: u64) -> Vec<PreparedRequest> {
    match args.rpc {
        RpcMode::UnaryBytes => (0..REQUEST_POOL_LEN)
            .map(|offset| {
                let payload = make_payload(
                    args.payload_size,
                    args.payload_kind,
                    worker_seed * 100 + offset as u64,
                );
                let request = EchoRequest {
                    body: Some(EchoRequestBody::Payload(payload.clone())),
                    requested_response_shape: ShapeKind::Unspecified as i32,
                };
                let reply = EchoReply {
                    body: Some(EchoReplyBody::Payload(payload.clone())),
                };
                PreparedRequest {
                    body: PreparedRequestBody::Bytes(payload),
                    expected_response_shape: None,
                    request_serialized_size: request.encoded_len(),
                    response_serialized_size: reply.encoded_len(),
                }
            })
            .collect(),
        RpcMode::UnaryProtoShape => {
            let request_shape = args.proto_shape.expect("proto shape required");
            let response_shape = resolve_response_shape(request_shape, args.response_shape);
            (0..REQUEST_POOL_LEN)
                .map(|offset| {
                    let request_message =
                        build_proto_message(request_shape, worker_seed * 100 + offset as u64);
                    let response_message = build_proto_message(
                        response_shape,
                        10_000 + worker_seed * 100 + offset as u64,
                    );
                    let request = EchoRequest {
                        body: Some(EchoRequestBody::ProtoPayload(request_message.clone())),
                        requested_response_shape: response_shape.to_wire(),
                    };
                    let reply = EchoReply {
                        body: Some(EchoReplyBody::ProtoPayload(response_message)),
                    };
                    PreparedRequest {
                        body: PreparedRequestBody::Proto(request_message),
                        expected_response_shape: Some(response_shape),
                        request_serialized_size: request.encoded_len(),
                        response_serialized_size: reply.encoded_len(),
                    }
                })
                .collect()
        }
    }
}

impl PreparedRequest {
    pub(crate) fn request(&self) -> EchoRequest {
        match &self.body {
            PreparedRequestBody::Bytes(payload) => EchoRequest {
                body: Some(EchoRequestBody::Payload(payload.clone())),
                requested_response_shape: ShapeKind::Unspecified as i32,
            },
            PreparedRequestBody::Proto(message) => EchoRequest {
                body: Some(EchoRequestBody::ProtoPayload(message.clone())),
                requested_response_shape: self
                    .expected_response_shape
                    .expect("proto workloads must have a response shape")
                    .to_wire(),
            },
        }
    }
}

pub(crate) fn validate_response(
    prepared: &PreparedRequest,
    response: &EchoReply,
) -> Result<usize, BoxError> {
    match (&prepared.body, response.body.as_ref()) {
        (PreparedRequestBody::Bytes(_), Some(EchoReplyBody::Payload(_))) => {
            Ok(response.encoded_len())
        }
        (PreparedRequestBody::Bytes(_), Some(EchoReplyBody::ProtoPayload(_))) => Err(
            "bytes workload received a proto response instead of an echoed bytes payload".into(),
        ),
        (PreparedRequestBody::Proto(_), Some(EchoReplyBody::Payload(_))) => {
            Err("proto-shape workload received a bytes response instead of a proto shape".into())
        }
        (PreparedRequestBody::Proto(_), Some(EchoReplyBody::ProtoPayload(message))) => {
            let actual = proto_shape_from_message(message).ok_or_else(|| {
                "proto-shape response body was missing a concrete shape".to_string()
            })?;
            let expected = prepared
                .expected_response_shape
                .ok_or_else(|| "proto workload missing expected response shape".to_string())?;
            if actual != expected {
                return Err(format!(
                    "response shape mismatch: expected {}, got {}",
                    serialize_proto_shape(expected),
                    serialize_proto_shape(actual)
                )
                .into());
            }
            let actual_size = response.encoded_len();
            if actual_size != prepared.response_serialized_size {
                return Err(format!(
                    "response serialized size mismatch for {}: expected {}, got {}",
                    serialize_proto_shape(expected),
                    prepared.response_serialized_size,
                    actual_size
                )
                .into());
            }
            Ok(actual_size)
        }
        (_, None) => Err("response body was missing".into()),
    }
}

fn make_payload(size: usize, kind: PayloadKind, seed: u64) -> Vec<u8> {
    match kind {
        PayloadKind::Random => {
            let mut buf = vec![0_u8; size];
            let mut rng = StdRng::seed_from_u64(seed + 1);
            rng.fill_bytes(&mut buf);
            buf
        }
        PayloadKind::Structured => {
            let mut buf = Vec::with_capacity(size);
            let pattern = format!(
                "{{\"id\":{},\"name\":\"tonic-profile\",\"flags\":[1,0,1],\"payload\":\"",
                seed
            );
            while buf.len() < size {
                buf.extend_from_slice(pattern.as_bytes());
                buf.extend_from_slice(b"abcdefghijklmnoqrstuvwxyz0123456789");
            }
            buf.truncate(size);
            buf
        }
        PayloadKind::Repeated => vec![b'R'; size],
    }
}

pub(crate) fn build_proto_message(shape: ProtoShape, seed: u64) -> CompactProfileShape {
    match shape {
        ProtoShape::FleetSmall => CompactProfileShape {
            shape: Some(profile::compact_profile_shape::Shape::FleetSmall(
                FleetSmallShape {
                    id: (seed % 32) + 1,
                    service: format!("svc-{:04}", seed % 10_000),
                    method: format!("unary-{:04}", seed % 10_000),
                    flags: vec![1, 0, 1, (seed % 3) as u32],
                    token: seeded_bytes(seed, 16),
                },
            )),
        },
        ProtoShape::FleetStringHeavy => CompactProfileShape {
            shape: Some(profile::compact_profile_shape::Shape::FleetStringHeavy(
                FleetStringHeavyShape {
                    id: (seed % 32) + 10,
                    labels: (0..6)
                        .map(|idx| fixed_string(&format!("label-{idx}-{}", seed % 97), 28))
                        .collect(),
                    paths: (0..4)
                        .map(|idx| fixed_string(&format!("/fleet/{idx}/{}", seed % 31), 36))
                        .collect(),
                    counters: (0..10).map(|idx| ((seed + idx as u64) % 64) + 1).collect(),
                    leaves: (0..4)
                        .map(|idx| FleetStringLeaf {
                            name: fixed_string(&format!("leaf-{idx}"), 18),
                            value: fixed_string(&format!("value-{}-{idx}", seed % 41), 40),
                            samples: (0..3)
                                .map(|sample| {
                                    fixed_string(
                                        &format!("sample-{idx}-{sample}-{}", seed % 53),
                                        26,
                                    )
                                })
                                .collect(),
                        })
                        .collect(),
                    note: fixed_string(&format!("note-{}", seed % 67), 52),
                },
            )),
        },
        ProtoShape::FleetResponseHeavy => CompactProfileShape {
            shape: Some(profile::compact_profile_shape::Shape::FleetResponseHeavy(
                FleetResponseHeavyShape {
                    tenant_id: (seed % 64) + 100,
                    trace_id: fixed_string(&format!("trace-{:016x}", seed + 100), 32),
                    regions: (0..6)
                        .map(|idx| fixed_string(&format!("region-{idx}-{}", seed % 13), 20))
                        .collect(),
                    entries: (0..12)
                        .map(|idx| FleetResponseEntry {
                            key: fixed_string(&format!("key-{idx}"), 16),
                            value: fixed_string(&format!("value-{idx}-{}", seed % 29), 48),
                            tags: (0..5)
                                .map(|tag| fixed_string(&format!("tag-{idx}-{tag}"), 18))
                                .collect(),
                            samples: (0..10)
                                .map(|sample| ((seed + idx as u64 * 10 + sample as u64) % 96) + 1)
                                .collect(),
                            digest: seeded_bytes(seed + idx as u64, 24),
                        })
                        .collect(),
                    digests: (0..10)
                        .map(|idx| seeded_bytes(seed + idx as u64 + 50, 20))
                        .collect(),
                    histogram: (0..32).map(|idx| ((seed + idx as u64) % 96) + 1).collect(),
                    summary: fixed_string(&format!("response-heavy-summary-{}", seed % 101), 96),
                },
            )),
        },
    }
}

fn fixed_string(seed: &str, len: usize) -> String {
    let mut output = String::with_capacity(len);
    while output.len() < len {
        output.push_str(seed);
        output.push('-');
    }
    output.truncate(len);
    output
}

fn seeded_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; len];
    let mut rng = StdRng::seed_from_u64(seed + 7);
    rng.fill_bytes(&mut bytes);
    bytes
}

pub(crate) fn proto_shape_from_message(message: &CompactProfileShape) -> Option<ProtoShape> {
    match message.shape.as_ref()? {
        profile::compact_profile_shape::Shape::FleetSmall(_) => Some(ProtoShape::FleetSmall),
        profile::compact_profile_shape::Shape::FleetStringHeavy(_) => {
            Some(ProtoShape::FleetStringHeavy)
        }
        profile::compact_profile_shape::Shape::FleetResponseHeavy(_) => {
            Some(ProtoShape::FleetResponseHeavy)
        }
    }
}

pub(crate) fn proto_shape_from_wire(value: i32) -> Result<Option<ProtoShape>, Status> {
    match ShapeKind::try_from(value).unwrap_or(ShapeKind::Unspecified) {
        ShapeKind::Unspecified => Ok(None),
        ShapeKind::FleetSmall => Ok(Some(ProtoShape::FleetSmall)),
        ShapeKind::FleetStringHeavy => Ok(Some(ProtoShape::FleetStringHeavy)),
        ShapeKind::FleetResponseHeavy => Ok(Some(ProtoShape::FleetResponseHeavy)),
    }
}

pub(crate) fn resolve_response_shape(
    request_shape: ProtoShape,
    response_shape: ResponseShape,
) -> ProtoShape {
    match response_shape {
        ResponseShape::Same => request_shape,
        ResponseShape::FleetSmall => ProtoShape::FleetSmall,
        ResponseShape::FleetStringHeavy => ProtoShape::FleetStringHeavy,
        ResponseShape::FleetResponseHeavy => ProtoShape::FleetResponseHeavy,
    }
}

impl ProtoShape {
    fn to_wire(self) -> i32 {
        match self {
            ProtoShape::FleetSmall => ShapeKind::FleetSmall as i32,
            ProtoShape::FleetStringHeavy => ShapeKind::FleetStringHeavy as i32,
            ProtoShape::FleetResponseHeavy => ShapeKind::FleetResponseHeavy as i32,
        }
    }
}

pub(crate) fn serialize_proto_shape(shape: ProtoShape) -> &'static str {
    match shape {
        ProtoShape::FleetSmall => "fleet-small",
        ProtoShape::FleetStringHeavy => "fleet-string-heavy",
        ProtoShape::FleetResponseHeavy => "fleet-response-heavy",
    }
}

pub(crate) fn serialize_payload_kind(kind: PayloadKind) -> &'static str {
    match kind {
        PayloadKind::Random => "random",
        PayloadKind::Structured => "structured",
        PayloadKind::Repeated => "repeated",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CompressionMode, InstrumentationMode, Mode, RuntimeMode};

    fn proto_args(response_shape: ResponseShape) -> Args {
        Args {
            mode: Mode::Selftest,
            rpc: RpcMode::UnaryProtoShape,
            proto_shape: Some(ProtoShape::FleetSmall),
            response_shape,
            bind: "127.0.0.1:50051".into(),
            target: "127.0.0.1:50051".into(),
            payload_size: 256,
            payload_kind: PayloadKind::Structured,
            compression: CompressionMode::Off,
            concurrency: 1,
            requests: Some(1),
            warmup_ms: 0,
            measure_ms: 1,
            runtime: RuntimeMode::Single,
            instrumentation: InstrumentationMode::On,
            accelerated_path: crate::cli::AcceleratedPath::Software,
            accelerator_device: None,
            accelerator_lane: None,
            buffer_policy: crate::cli::BufferPolicy::Default,
            server_core: None,
            client_core: None,
            json_out: None,
            server_json_out: None,
            run_id: None,
            shutdown_after_requests: None,
        }
    }

    #[test]
    fn proto_response_validation_rejects_missing_body() {
        let prepared = build_request_pool(&proto_args(ResponseShape::FleetResponseHeavy), 0)
            .into_iter()
            .next()
            .expect("prepared request");
        let err = validate_response(&prepared, &EchoReply { body: None }).expect_err("missing body should fail");
        assert!(
            err.to_string().contains("response body was missing"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn proto_response_validation_rejects_shape_mismatch() {
        let prepared = build_request_pool(&proto_args(ResponseShape::FleetResponseHeavy), 0)
            .into_iter()
            .next()
            .expect("prepared request");
        let mismatched = EchoReply {
            body: Some(EchoReplyBody::ProtoPayload(build_proto_message(
                ProtoShape::FleetStringHeavy,
                42,
            ))),
        };
        let err = validate_response(&prepared, &mismatched).expect_err("shape mismatch should fail");
        assert!(
            err.to_string().contains("response shape mismatch"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn same_shape_workload_probe_keeps_small_size_delta() {
        let descriptor = workload_probe(&proto_args(ResponseShape::Same));
        assert_eq!(descriptor.selection_policy, SelectionPolicy::SameAsRequest);
        assert_eq!(descriptor.request_shape, Some(ProtoShape::FleetSmall));
        assert_eq!(descriptor.response_shape, Some(ProtoShape::FleetSmall));
        assert!(descriptor.request_serialized_size >= descriptor.response_serialized_size);
        assert!(descriptor.request_serialized_size - descriptor.response_serialized_size <= 8);
    }
}
