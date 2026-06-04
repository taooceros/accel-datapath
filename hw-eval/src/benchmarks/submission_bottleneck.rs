mod common;
mod experiment_1_submit_occupancy;
mod experiment_2_marker_overlap;
mod experiment_3_traffic_ladder;
mod experiment_4_completion_reuse;
mod experiment_5_submit_admission_probe;

use hw_eval::submit::WqPortal;

use crate::config::{BenchmarkKind, SubmissionBottleneckConfig};
use crate::report::SubmissionBottleneckResults;

use experiment_1_submit_occupancy::bench_submit_occupancy_one_extra;
use experiment_2_marker_overlap::{
    bench_submit_marker_mechanism_probes, bench_submit_marker_overlap,
};
use experiment_3_traffic_ladder::bench_traffic_class_ladder;
use experiment_4_completion_reuse::bench_completion_reuse_policy;
use experiment_5_submit_admission_probe::bench_submit_admission_probe;

pub(crate) struct BottleneckRequest<'a> {
    pub(crate) wq: &'a WqPortal,
    pub(crate) benchmark: BenchmarkKind,
    pub(crate) config: &'a SubmissionBottleneckConfig,
    pub(crate) submit_bursts: &'a [usize],
    pub(crate) iterations: usize,
    pub(crate) tsc_freq: u64,
    pub(crate) json: bool,
}

pub(crate) fn run(
    request: BottleneckRequest<'_>,
    results: &mut SubmissionBottleneckResults,
) -> bool {
    match request.benchmark {
        BenchmarkKind::SubmitAdmission => {
            bench_submit_admission_probe(
                request.wq,
                request.iterations,
                request.submit_bursts,
                request.tsc_freq,
                request.json,
                &mut results.admission,
            );
            true
        }
        BenchmarkKind::SubmitOccupancy => {
            bench_submit_occupancy_one_extra(
                request.wq,
                &request.config.submit_occupancies,
                request.config.dsa_operation,
                request.config.dsa_payload_size,
                request.config.submit_occupancy_trace_until,
                request.config.submit_occupancy_spin_iters,
                request.config.submit_occupancy_gap_tsc,
                request.config.submit_occupancy_shared_payload,
                request.iterations,
                request.tsc_freq,
                request.json,
                &mut results.submit_occupancy,
            );
            true
        }
        BenchmarkKind::SubmitMarkerOverlap => {
            bench_submit_marker_overlap(
                request.wq,
                &request.config.marker_bursts,
                &request.config.marker_positions,
                &request.config.marker_poll_offsets,
                request.config.dsa_operation,
                request.config.dsa_payload_size,
                request.iterations,
                request.tsc_freq,
                request.json,
                &mut results.submit_marker_overlap,
            );
            true
        }
        BenchmarkKind::SubmitMarkerMechanism => {
            bench_submit_marker_mechanism_probes(
                request.wq,
                &request.config.marker_bursts,
                &request.config.marker_poll_offsets,
                &request.config.marker_poll_submit_batches,
                request.config.dsa_operation,
                request.config.dsa_payload_size,
                request.iterations,
                request.tsc_freq,
                request.json,
                &mut results.submit_marker_mechanism,
            );
            true
        }
        BenchmarkKind::TrafficClassLadder => {
            bench_traffic_class_ladder(
                request.wq,
                &request.config.traffic_windows,
                &request.config.traffic_classes,
                request.iterations,
                request.tsc_freq,
                request.json,
                &mut results.traffic_class_ladder,
            );
            true
        }
        BenchmarkKind::CompletionReusePolicy => {
            bench_completion_reuse_policy(
                request.wq,
                &request.config.completion_reuse_policies,
                request.config.completion_reuse_window,
                request.config.dsa_operation,
                request.iterations,
                request.tsc_freq,
                request.json,
                &mut results.completion_reuse_policy,
            );
            true
        }
        _ => false,
    }
}
