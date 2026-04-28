use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageKind {
    Encode,
    Decode,
    Compress,
    Decompress,
    BufferReserve,
    BodyAccum,
    FrameHeader,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageEvent {
    pub stage: StageKind,
    pub count: u64,
    pub nanos: u64,
    pub bytes: u64,
}

#[derive(Clone, Copy, Default)]
pub struct Counter {
    pub count: u64,
    pub nanos: u64,
    pub bytes: u64,
}

#[derive(Clone, Copy, Default)]
pub struct Snapshot {
    pub enabled: bool,
    pub encode: Counter,
    pub decode: Counter,
    pub compress: Counter,
    pub decompress: Counter,
    pub buffer_reserve: Counter,
    pub body_accum: Counter,
    pub frame_header: Counter,
}

#[derive(Default)]
struct CounterState {
    count: AtomicU64,
    nanos: AtomicU64,
    bytes: AtomicU64,
}

impl CounterState {
    fn reset(&self) {
        self.count.store(0, Ordering::Relaxed);
        self.nanos.store(0, Ordering::Relaxed);
        self.bytes.store(0, Ordering::Relaxed);
    }

    fn snapshot(&self) -> Counter {
        Counter {
            count: self.count.load(Ordering::Relaxed),
            nanos: self.nanos.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
        }
    }

    fn record(&self, event: StageEvent) {
        self.count.fetch_add(event.count, Ordering::Relaxed);
        self.nanos.fetch_add(event.nanos, Ordering::Relaxed);
        self.bytes.fetch_add(event.bytes, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct State {
    encode: CounterState,
    decode: CounterState,
    compress: CounterState,
    decompress: CounterState,
    buffer_reserve: CounterState,
    body_accum: CounterState,
    frame_header: CounterState,
}

impl State {
    fn counter(&self, stage: StageKind) -> &CounterState {
        match stage {
            StageKind::Encode => &self.encode,
            StageKind::Decode => &self.decode,
            StageKind::Compress => &self.compress,
            StageKind::Decompress => &self.decompress,
            StageKind::BufferReserve => &self.buffer_reserve,
            StageKind::BodyAccum => &self.body_accum,
            StageKind::FrameHeader => &self.frame_header,
        }
    }

    fn reset(&self) {
        self.encode.reset();
        self.decode.reset();
        self.compress.reset();
        self.decompress.reset();
        self.buffer_reserve.reset();
        self.body_accum.reset();
        self.frame_header.reset();
    }

    fn snapshot(&self, enabled: bool) -> Snapshot {
        Snapshot {
            enabled,
            encode: self.encode.snapshot(),
            decode: self.decode.snapshot(),
            compress: self.compress.snapshot(),
            decompress: self.decompress.snapshot(),
            buffer_reserve: self.buffer_reserve.snapshot(),
            body_accum: self.body_accum.snapshot(),
            frame_header: self.frame_header.snapshot(),
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static STATE: OnceLock<State> = OnceLock::new();

fn state() -> &'static State {
    STATE.get_or_init(State::default)
}

pub fn record_stage(stage: StageKind, bytes: usize, nanos: u64) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }

    state().counter(stage).record(StageEvent {
        stage,
        count: 1,
        nanos,
        bytes: bytes as u64,
    });
}

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn reset() {
    state().reset();
}

pub fn snapshot() -> Snapshot {
    state().snapshot(ENABLED.load(Ordering::Relaxed))
}
