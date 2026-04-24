use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};

use prost::Message;
use tonic::codec::{BufferSettings, Codec};
use tonic_prost::ProstCodec;

pub const DEFAULT_CODEC_BUFFER_SIZE: usize = 8 * 1024;
pub const DEFAULT_CODEC_YIELD_THRESHOLD: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveBufferSettings {
    pub buffer_size: usize,
    pub yield_threshold: usize,
}

impl EffectiveBufferSettings {
    pub const fn new(buffer_size: usize, yield_threshold: usize) -> Self {
        Self {
            buffer_size,
            yield_threshold,
        }
    }

    pub const fn defaults() -> Self {
        Self::new(DEFAULT_CODEC_BUFFER_SIZE, DEFAULT_CODEC_YIELD_THRESHOLD)
    }

    fn as_tonic(self) -> BufferSettings {
        BufferSettings::new(self.buffer_size, self.yield_threshold)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecObservation {
    pub buffer_size: usize,
    pub yield_threshold: usize,
    pub encoder_count: usize,
    pub decoder_count: usize,
}

static ACTIVE_SETTINGS: OnceLock<RwLock<EffectiveBufferSettings>> = OnceLock::new();
static OBSERVED_BUFFER_SIZE: AtomicUsize = AtomicUsize::new(0);
static OBSERVED_YIELD_THRESHOLD: AtomicUsize = AtomicUsize::new(0);
static ENCODER_COUNT: AtomicUsize = AtomicUsize::new(0);
static DECODER_COUNT: AtomicUsize = AtomicUsize::new(0);

fn active_settings_cell() -> &'static RwLock<EffectiveBufferSettings> {
    ACTIVE_SETTINGS.get_or_init(|| RwLock::new(EffectiveBufferSettings::defaults()))
}

pub fn set_process_default_buffer_settings(
    buffer_size: Option<usize>,
    yield_threshold: Option<usize>,
) -> Result<EffectiveBufferSettings, String> {
    let settings = match (buffer_size, yield_threshold) {
        (None, None) => EffectiveBufferSettings::defaults(),
        (Some(buffer_size), Some(yield_threshold)) if buffer_size > 0 && yield_threshold > 0 => {
            EffectiveBufferSettings::new(buffer_size, yield_threshold)
        }
        (Some(_), Some(_)) => {
            return Err("codec buffer settings must be strictly positive".to_string())
        }
        _ => {
            return Err(
                "codec buffer settings must provide both buffer_size and yield_threshold"
                    .to_string(),
            )
        }
    };

    *active_settings_cell()
        .write()
        .expect("active codec settings lock poisoned") = settings;
    Ok(settings)
}

pub fn configured_settings() -> EffectiveBufferSettings {
    *active_settings_cell()
        .read()
        .expect("active codec settings lock poisoned")
}

pub fn reset_observations() {
    OBSERVED_BUFFER_SIZE.store(0, Ordering::Relaxed);
    OBSERVED_YIELD_THRESHOLD.store(0, Ordering::Relaxed);
    ENCODER_COUNT.store(0, Ordering::Relaxed);
    DECODER_COUNT.store(0, Ordering::Relaxed);
}

pub fn observed_settings() -> Option<CodecObservation> {
    let encoder_count = ENCODER_COUNT.load(Ordering::Relaxed);
    let decoder_count = DECODER_COUNT.load(Ordering::Relaxed);
    if encoder_count == 0 || decoder_count == 0 {
        return None;
    }

    Some(CodecObservation {
        buffer_size: OBSERVED_BUFFER_SIZE.load(Ordering::Relaxed),
        yield_threshold: OBSERVED_YIELD_THRESHOLD.load(Ordering::Relaxed),
        encoder_count,
        decoder_count,
    })
}

fn record_observation(settings: EffectiveBufferSettings, encoder: bool) {
    OBSERVED_BUFFER_SIZE.store(settings.buffer_size, Ordering::Relaxed);
    OBSERVED_YIELD_THRESHOLD.store(settings.yield_threshold, Ordering::Relaxed);
    if encoder {
        ENCODER_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        DECODER_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProfileCodec<T, U>(PhantomData<(T, U)>);

impl<T, U> Codec for ProfileCodec<T, U>
where
    T: Message + Send + 'static,
    U: Message + Default + Send + 'static,
{
    type Encode = T;
    type Decode = U;

    type Encoder = <ProstCodec<T, U> as Codec>::Encoder;
    type Decoder = <ProstCodec<T, U> as Codec>::Decoder;

    fn encoder(&mut self) -> Self::Encoder {
        let settings = configured_settings();
        record_observation(settings, true);
        ProstCodec::<T, U>::raw_encoder(settings.as_tonic())
    }

    fn decoder(&mut self) -> Self::Decoder {
        let settings = configured_settings();
        record_observation(settings, false);
        ProstCodec::<T, U>::raw_decoder(settings.as_tonic())
    }
}
