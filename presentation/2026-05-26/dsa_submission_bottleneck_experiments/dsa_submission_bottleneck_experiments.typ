// Diagnostic experiment deck: DSA submission bottleneck localization.
// Reader: advisor / project collaborator.
// Claim boundary: experiment design plus the first measured submit-occupancy result.
// Sources:
// - docs/plan/2026-05-26/01.dsa-submission-bottleneck-experiment-slide.plan.md
// - docs/report/benchmarking/018.dsa_submit_workload_study_2026-05-24.md
// - docs/report/benchmarking/019.submit_occupancy_one_extra_2026-05-26.md
// - docs/report/literature/papers/understanding-the-host-network/paper.md
// - docs/report/literature/005.accelerator_hostpath_2026-03-28.md
// - Direct latency measurement in the 2026-05-26 working session.

#import "support.typ": deck
#import "topics/introduction.typ" as introduction
#import "topics/experiment_1_submit_occupancy.typ" as experiment_1
#import "topics/experiment_2_marker_overlap.typ" as experiment_2
#import "topics/experiment_3_traffic_ladder.typ" as experiment_3
#import "topics/experiment_4_completion_reuse.typ" as experiment_4
#import "topics/experiment_5_blind_push.typ" as experiment_5
#import "topics/attribution.typ" as attribution

#show: deck.with(
  margin: (x: 44pt, y: 32pt),
  size: 13.2pt,
  leading: 0.84em,
  spacing: 0.54em,
)

#introduction.title()
#introduction.host_path_view()
#introduction.dsa_loop_timestamps()
#introduction.dsa_traffic_classes()
#experiment_1.setup()
#experiment_1.why_run_it()
#experiment_2.setup()
#experiment_2.why_run_it()
#experiment_2.polling_control()
#experiment_3.setup()
#experiment_3.why_run_it()
#experiment_4.setup()
#experiment_4.why_run_it()
#experiment_5.setup()
#experiment_5.why_run_it()
#attribution.rule()
#experiment_1.result()
