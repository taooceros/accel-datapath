use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::Instant;

use bytes::{BufMut, BytesMut};
use idxd_rust::{DsaSession, MemmoveError, MemmovePhase};
use prost::Message;
use tonic::codec::instrumentation::{record_stage, StageKind};
use tonic::codec::{BufferSettings, Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::Status;
use tonic_prost::ProstDecoder;

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
pub enum AcceleratedCopyPath {
    Software,
    Idxd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveAccelerationSettings {
    pub selected_path: AcceleratedCopyPath,
    pub device_path: Option<PathBuf>,
}

impl EffectiveAccelerationSettings {
    pub fn software() -> Self {
        Self {
            selected_path: AcceleratedCopyPath::Software,
            device_path: None,
        }
    }

    pub fn idxd(device_path: PathBuf) -> Self {
        Self {
            selected_path: AcceleratedCopyPath::Idxd,
            device_path: Some(device_path),
        }
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
static ACTIVE_ACCELERATION: OnceLock<RwLock<EffectiveAccelerationSettings>> = OnceLock::new();
static OBSERVED_BUFFER_SIZE: AtomicUsize = AtomicUsize::new(0);
static OBSERVED_YIELD_THRESHOLD: AtomicUsize = AtomicUsize::new(0);
static ENCODER_COUNT: AtomicUsize = AtomicUsize::new(0);
static DECODER_COUNT: AtomicUsize = AtomicUsize::new(0);

fn active_settings_cell() -> &'static RwLock<EffectiveBufferSettings> {
    ACTIVE_SETTINGS.get_or_init(|| RwLock::new(EffectiveBufferSettings::defaults()))
}

fn active_acceleration_cell() -> &'static RwLock<EffectiveAccelerationSettings> {
    ACTIVE_ACCELERATION.get_or_init(|| RwLock::new(EffectiveAccelerationSettings::software()))
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

pub fn set_process_default_acceleration(
    selected_path: AcceleratedCopyPath,
    device_path: Option<PathBuf>,
) -> Result<EffectiveAccelerationSettings, String> {
    let settings = match selected_path {
        AcceleratedCopyPath::Software => {
            if device_path.is_some() {
                return Err(
                    "software codec acceleration must not provide an accelerator device"
                        .to_string(),
                );
            }
            EffectiveAccelerationSettings::software()
        }
        AcceleratedCopyPath::Idxd => {
            let device_path = device_path.ok_or_else(|| {
                "idxd codec acceleration requires an explicit accelerator device".to_string()
            })?;
            EffectiveAccelerationSettings::idxd(device_path)
        }
    };

    *active_acceleration_cell()
        .write()
        .expect("active codec acceleration lock poisoned") = settings.clone();
    Ok(settings)
}

pub fn configured_settings() -> EffectiveBufferSettings {
    *active_settings_cell()
        .read()
        .expect("active codec settings lock poisoned")
}

pub fn configured_acceleration() -> EffectiveAccelerationSettings {
    active_acceleration_cell()
        .read()
        .expect("active codec acceleration lock poisoned")
        .clone()
}

pub fn preflight_acceleration() -> Result<(), String> {
    let acceleration = configured_acceleration();
    if acceleration.selected_path == AcceleratedCopyPath::Software {
        return Ok(());
    }

    let device_path = acceleration
        .device_path
        .ok_or_else(|| "idxd codec acceleration missing device path".to_string())?;
    DsaSession::open(&device_path)
        .map(|_| ())
        .map_err(|err| idxd_status(&device_path, &err).message().to_string())
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

    type Encoder = ProfileEncoder<T>;
    type Decoder = ProfileDecoder<U>;

    fn encoder(&mut self) -> Self::Encoder {
        let settings = configured_settings();
        record_observation(settings, true);
        ProfileEncoder::new(settings, configured_acceleration())
    }

    fn decoder(&mut self) -> Self::Decoder {
        let settings = configured_settings();
        record_observation(settings, false);
        ProfileDecoder::<U>::new(settings)
    }
}

pub struct ProfileEncoder<T> {
    _pd: PhantomData<T>,
    buffer_settings: BufferSettings,
    acceleration: EffectiveAccelerationSettings,
    staging: BytesMut,
    scratch: Vec<u8>,
    session: Option<DsaSession>,
}

impl<T> ProfileEncoder<T> {
    fn new(
        settings: EffectiveBufferSettings,
        acceleration: EffectiveAccelerationSettings,
    ) -> Self {
        let buffer_settings = settings.as_tonic();
        Self {
            _pd: PhantomData,
            buffer_settings,
            acceleration,
            staging: BytesMut::with_capacity(settings.buffer_size),
            scratch: Vec::new(),
            session: None,
        }
    }

    fn ensure_session(&mut self) -> Result<(), Status> {
        if self.session.is_none() {
            let device_path = self
                .acceleration
                .device_path
                .clone()
                .ok_or_else(|| Status::internal("idxd codec acceleration missing device path"))?;
            let session = DsaSession::open(&device_path)
                .map_err(|err| idxd_status(&device_path, &err))?;
            self.session = Some(session);
        }
        Ok(())
    }
}

impl<T: Message> Encoder for ProfileEncoder<T> {
    type Item = T;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        let encoded_len = item.encoded_len();
        let started = Instant::now();

        match self.acceleration.selected_path {
            AcceleratedCopyPath::Software => {
                item.encode(dst)
                    .expect("Message only errors if not enough space");
            }
            AcceleratedCopyPath::Idxd => {
                if encoded_len == 0 {
                    return Err(Status::internal(
                        "idxd codec copy lane rejected a zero-length encoded frame",
                    ));
                }

                self.staging.clear();
                self.staging.reserve(encoded_len);
                item.encode(&mut self.staging)
                    .expect("Message only errors if not enough space");

                if self.staging.len() != encoded_len {
                    return Err(Status::internal(format!(
                        "idxd codec copy lane encoded {} bytes but staging held {} bytes",
                        encoded_len,
                        self.staging.len()
                    )));
                }

                self.scratch.resize(encoded_len, 0);
                self.ensure_session()?;
                let device_path = self
                    .acceleration
                    .device_path
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("<missing-device>"));
                let session = self.session.as_ref().expect("session populated");
                session
                    .memmove(&mut self.scratch, &self.staging)
                    .map_err(|err| idxd_status(&device_path, &err))?;

                dst.put_slice(&self.scratch);
            }
        }

        record_stage(
            StageKind::Encode,
            encoded_len,
            started.elapsed().as_nanos() as u64,
        );
        Ok(())
    }

    fn buffer_settings(&self) -> BufferSettings {
        self.buffer_settings
    }
}

#[derive(Debug)]
pub struct ProfileDecoder<U> {
    inner: ProstDecoder<U>,
}

impl<U> ProfileDecoder<U> {
    fn new(settings: EffectiveBufferSettings) -> Self {
        Self {
            inner: ProstDecoder::<U>::new(settings.as_tonic()),
        }
    }
}

impl<U: Message + Default> Decoder for ProfileDecoder<U> {
    type Item = U;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode(src)
    }

    fn buffer_settings(&self) -> BufferSettings {
        self.inner.buffer_settings()
    }
}

fn idxd_status(device_path: &Path, err: &MemmoveError) -> Status {
    let phase = match err {
        MemmoveError::InvalidDevicePath { .. } => "invalid_device_path",
        MemmoveError::InvalidLength { .. } | MemmoveError::DestinationTooSmall { .. } => {
            "copy_validation"
        }
        MemmoveError::QueueOpen { phase, .. }
        | MemmoveError::CompletionTimeout { phase, .. }
        | MemmoveError::MalformedCompletion { phase, .. }
        | MemmoveError::PageFaultRetryExhausted { phase, .. }
        | MemmoveError::CompletionStatus { phase, .. }
        | MemmoveError::ByteMismatch { phase, .. } => phase_label(*phase),
    };
    let kind = err.kind();

    Status::internal(format!(
        "idxd codec copy lane failure during {phase} ({kind}) on {}: {err}",
        device_path.display()
    ))
}

fn phase_label(phase: MemmovePhase) -> &'static str {
    match phase {
        MemmovePhase::QueueOpen => "queue_open",
        MemmovePhase::CompletionPoll => "completion_poll",
        MemmovePhase::PageFaultRetry => "page_fault_retry",
        MemmovePhase::PostCopyVerify => "post_copy_verify",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_post_copy_verification_failures_without_collapsing_phase() {
        let status = idxd_status(
            Path::new("/dev/dsa/wq0.0"),
            &MemmoveError::ByteMismatch {
                device_path: PathBuf::from("/dev/dsa/wq0.0"),
                phase: MemmovePhase::PostCopyVerify,
                requested_bytes: 128,
                mismatch_offset: 17,
                final_status: 0,
                page_fault_retries: 1,
            },
        );

        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(status.message().contains("during post_copy_verify (byte_mismatch)"));
        assert!(status.message().contains("mismatch_offset=17"));
    }

    #[test]
    fn translates_setup_failures_with_specific_setup_label() {
        let status = idxd_status(
            Path::new("<configured-device>"),
            &MemmoveError::InvalidDevicePath {
                device_path: PathBuf::from(""),
            },
        );

        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(status.message().contains("during invalid_device_path (invalid_device_path)"));
    }

    #[test]
    fn preserves_page_fault_retry_label_instead_of_completion_poll() {
        let status = idxd_status(
            Path::new("/dev/dsa/wq0.0"),
            &MemmoveError::PageFaultRetryExhausted {
                device_path: PathBuf::from("/dev/dsa/wq0.0"),
                phase: MemmovePhase::PageFaultRetry,
                retries: 1,
                bytes_completed: 32,
                fault_addr: 0xfeed,
            },
        );

        assert!(status
            .message()
            .contains("during page_fault_retry (page_fault_retry_exhausted)"));
    }

    #[test]
    fn translates_all_runtime_phase_variants_with_phase_and_kind_context() {
        let device_path = Path::new("/dev/dsa/wq0.0");
        let runtime_cases = [
            (
                MemmoveError::QueueOpen {
                    device_path: device_path.to_path_buf(),
                    phase: MemmovePhase::QueueOpen,
                    source: std::io::Error::other("open failed"),
                },
                "during queue_open (queue_open)",
            ),
            (
                MemmoveError::CompletionTimeout {
                    device_path: device_path.to_path_buf(),
                    phase: MemmovePhase::CompletionPoll,
                    page_fault_retries: 0,
                },
                "during completion_poll (completion_timeout)",
            ),
            (
                MemmoveError::MalformedCompletion {
                    device_path: device_path.to_path_buf(),
                    phase: MemmovePhase::CompletionPoll,
                    status: 0x13,
                    result: 0,
                    bytes_completed: 48,
                    fault_addr: 0,
                    page_fault_retries: 1,
                    detail: "completion record missing bytes-valid flag",
                },
                "during completion_poll (malformed_completion)",
            ),
            (
                MemmoveError::CompletionStatus {
                    device_path: device_path.to_path_buf(),
                    phase: MemmovePhase::CompletionPoll,
                    status: 0x3,
                    bytes_completed: 24,
                    fault_addr: 0xfeed,
                    page_fault_retries: 2,
                },
                "during completion_poll (completion_status)",
            ),
            (
                MemmoveError::PageFaultRetryExhausted {
                    device_path: device_path.to_path_buf(),
                    phase: MemmovePhase::PageFaultRetry,
                    retries: 3,
                    bytes_completed: 64,
                    fault_addr: 0xfeed,
                },
                "during page_fault_retry (page_fault_retry_exhausted)",
            ),
            (
                MemmoveError::ByteMismatch {
                    device_path: device_path.to_path_buf(),
                    phase: MemmovePhase::PostCopyVerify,
                    requested_bytes: 128,
                    mismatch_offset: 11,
                    final_status: 0,
                    page_fault_retries: 1,
                },
                "during post_copy_verify (byte_mismatch)",
            ),
        ];

        for (err, expected_context) in runtime_cases {
            let status = idxd_status(device_path, &err);
            assert_eq!(status.code(), tonic::Code::Internal);
            assert!(
                status.message().contains(expected_context),
                "status message missing {expected_context}: {}",
                status.message()
            );
        }
    }

    #[test]
    fn preserves_copy_validation_bucket_for_preflight_length_errors() {
        let device_path = Path::new("/dev/dsa/wq0.0");
        let invalid_length = idxd_status(
            device_path,
            &MemmoveError::InvalidLength {
                requested_len: 0,
                max_len: 4096,
            },
        );
        let destination_too_small = idxd_status(
            device_path,
            &MemmoveError::DestinationTooSmall {
                src_len: 256,
                dst_len: 128,
            },
        );

        for status in [&invalid_length, &destination_too_small] {
            assert!(
                status.message().contains("during copy_validation"),
                "unexpected status message: {}",
                status.message()
            );
        }
        assert!(invalid_length
            .message()
            .contains("(invalid_length)"));
        assert!(destination_too_small
            .message()
            .contains("(destination_too_small)"));
    }
}
