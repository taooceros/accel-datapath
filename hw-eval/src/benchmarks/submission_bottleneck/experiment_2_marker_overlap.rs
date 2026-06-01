// Experiment 2: completion visibility across the submit wall.
//
// The concrete overlap baseline and mechanism-probe implementations live in
// private child modules. This outer module is the Experiment 2 facade: callers
// import benchmark entry methods from here instead of depending on the internal
// layout.

mod mechanism_probes;
mod overlap;

pub(crate) use mechanism_probes::bench_submit_marker_mechanism_probes;
pub(crate) use overlap::bench_submit_marker_overlap;
