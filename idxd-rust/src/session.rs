use std::io;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use idxd_sys::WqPortal;
use snafu::Snafu;

mod sealed {
    pub trait Sealed {}
}

/// Sealed marker trait for IDXD accelerator families supported by the generic session seam.
///
/// The concrete accelerator family is known at compile time. External crates can use the
/// provided marker types, but cannot implement new accelerator families until the crate grows
/// the corresponding descriptor and operation support deliberately.
pub trait Accelerator: sealed::Sealed + Copy + Clone + Default + std::fmt::Debug + 'static {
    /// Stable lowercase family name used in diagnostics.
    const NAME: &'static str;
    /// Default work-queue device path for this accelerator family.
    const DEFAULT_DEVICE_PATH: &'static str;
}

/// Intel Data Streaming Accelerator marker family.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dsa;

impl sealed::Sealed for Dsa {}

impl Accelerator for Dsa {
    const NAME: &'static str = "dsa";
    const DEFAULT_DEVICE_PATH: &'static str = "/dev/dsa/wq0.0";
}

/// Intel In-Memory Analytics Accelerator marker family.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Iax;

impl sealed::Sealed for Iax {}

impl Accelerator for Iax {
    const NAME: &'static str = "iax";
    const DEFAULT_DEVICE_PATH: &'static str = "/dev/iax/wq1.0";
}

/// Compatibility spelling for Intel IAA; the repo's low-level helpers use `Iax`.
pub type Iaa = Iax;

/// Configuration for opening one generic IDXD work-queue session.
///
/// This first seam owns only the family marker and device path. Operation-level details such as
/// memmove retry budgets remain on the existing DSA submission APIs until shared DSA/IAX
/// operation lifecycles prove they belong at session scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdxdSessionConfig<Accel: Accelerator> {
    device_path: PathBuf,
    accelerator: PhantomData<Accel>,
}

impl<Accel: Accelerator> Default for IdxdSessionConfig<Accel> {
    fn default() -> Self {
        Self {
            device_path: PathBuf::from(Accel::DEFAULT_DEVICE_PATH),
            accelerator: PhantomData,
        }
    }
}

impl<Accel: Accelerator> IdxdSessionConfig<Accel> {
    /// Build a config for an explicit work-queue device path.
    pub fn new<P: AsRef<Path>>(device_path: P) -> Result<Self, IdxdSessionError> {
        Ok(Self {
            device_path: normalize_device_path::<Accel>(device_path.as_ref())?,
            accelerator: PhantomData,
        })
    }

    /// Return the accelerator family name encoded by this config's marker type.
    pub fn accelerator_name(&self) -> &'static str {
        Accel::NAME
    }

    /// Return the normalized work-queue device path.
    pub fn device_path(&self) -> &Path {
        &self.device_path
    }
}

/// Generic first-version IDXD session seam over one mapped work queue.
///
/// `IdxdSession<Accel>` is intentionally construction-only in S01: it opens and owns one
/// `idxd_sys::WqPortal`, exposes queue metadata, and leaves concrete accelerator operations to
/// later slices. Existing `DsaSession` and `AsyncDsaSession` remain the live DSA memmove paths.
pub struct IdxdSession<Accel: Accelerator> {
    config: IdxdSessionConfig<Accel>,
    portal: WqPortal,
}

impl<Accel: Accelerator> IdxdSession<Accel> {
    /// Open one work queue for the accelerator family encoded in `Accel`.
    pub fn open<P: AsRef<Path>>(device_path: P) -> Result<Self, IdxdSessionError> {
        Self::open_config(IdxdSessionConfig::<Accel>::new(device_path)?)
    }

    /// Open one work queue from an already validated generic session config.
    pub fn open_config(config: IdxdSessionConfig<Accel>) -> Result<Self, IdxdSessionError> {
        let portal =
            WqPortal::open(config.device_path()).map_err(|source| IdxdSessionError::QueueOpen {
                accelerator_name: Accel::NAME,
                device_path: config.device_path().to_path_buf(),
                source,
            })?;

        Ok(Self { config, portal })
    }

    /// Return the accelerator family name encoded by this session's marker type.
    pub fn accelerator_name(&self) -> &'static str {
        self.config.accelerator_name()
    }

    /// Return the work-queue device path used to open this session.
    pub fn device_path(&self) -> &Path {
        self.config.device_path()
    }

    /// Return whether the opened work queue was detected as dedicated.
    pub fn is_dedicated_wq(&self) -> bool {
        self.portal.is_dedicated()
    }
}

/// Narrow failure surface for opening the generic IDXD session seam.
#[derive(Debug, Snafu)]
pub enum IdxdSessionError {
    /// The supplied work-queue path was malformed before queue-open was attempted.
    #[snafu(display(
        "invalid {accelerator_name} work-queue path: {}",
        device_path.display()
    ))]
    InvalidDevicePath {
        /// Accelerator family that rejected the path.
        accelerator_name: &'static str,
        /// Caller-supplied path that failed validation.
        device_path: PathBuf,
    },

    /// Opening the work queue failed in the OS or `idxd-sys` portal layer.
    #[snafu(display(
        "failed to open {accelerator_name} work queue {}: {source}",
        device_path.display()
    ))]
    QueueOpen {
        /// Accelerator family requested by the session marker type.
        accelerator_name: &'static str,
        /// Work-queue path passed to `idxd_sys::WqPortal::open`.
        device_path: PathBuf,
        /// Preserved OS queue-open source error.
        source: io::Error,
    },
}

impl IdxdSessionError {
    /// Stable machine-readable error kind for tests and operator diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidDevicePath { .. } => "invalid_device_path",
            Self::QueueOpen { .. } => "queue_open",
        }
    }

    /// Accelerator family associated with this open failure.
    pub fn accelerator_name(&self) -> &'static str {
        match self {
            Self::InvalidDevicePath {
                accelerator_name, ..
            }
            | Self::QueueOpen {
                accelerator_name, ..
            } => accelerator_name,
        }
    }

    /// Work-queue device path associated with this open failure.
    pub fn device_path(&self) -> Option<&Path> {
        match self {
            Self::InvalidDevicePath { device_path, .. } | Self::QueueOpen { device_path, .. } => {
                Some(device_path.as_path())
            }
        }
    }
}

fn normalize_device_path<Accel: Accelerator>(
    device_path: &Path,
) -> Result<PathBuf, IdxdSessionError> {
    let normalized = device_path.to_path_buf();
    if normalized.as_os_str().is_empty() {
        return Err(IdxdSessionError::InvalidDevicePath {
            accelerator_name: Accel::NAME,
            device_path: normalized,
        });
    }

    Ok(normalized)
}
