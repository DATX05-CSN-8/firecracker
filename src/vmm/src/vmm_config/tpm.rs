
use std::fmt;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use devices::virtio::tpm::Tpm;

type MutexTpm = Arc<Mutex<Tpm>>;

/// Errors associated with TPM config errors
#[derive(Debug, derive_more::From)]
pub enum TpmConfigError {
    /// General TPM config error, TODO change // todo ta bort helt kanske AAA
    GeneralTpmError,
    /// Cannot create tpm device
    CreateTpmDevice(String), // TODO AAA kolla vsock.rs VsockError
    /// Missing path for TPM device
    ParseTpmPathMissing,
}

impl fmt::Display for TpmConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::TpmConfigError::*;
        match self {
            GeneralTpmError => {
                write!(f, "General TPM Error!")
            }, // TODO remove
            CreateTpmDevice(err) => write!(f, "Failed to create TPM Device: {:?}", err),
            TpmConfigError::ParseTpmPathMissing => write!(f, "Error parsing --tpm: path missing"),
        }
    }
}

type Result<T> = std::result::Result<T, TpmConfigError>;

/// Used for describing the TPM Configuration
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TpmDeviceConfig {
    /// Path to the socket to be used
    pub socket: String
}

/// A builder of Tpm with Unix backend from 'TpmDeviceConfig'.
#[derive(Default)]
pub struct TpmBuilder {
    inner: Option<MutexTpm>,
}

impl TpmBuilder {
    
    /// Inserts a Tpm device in the store.
    pub fn set(&mut self, config: TpmDeviceConfig) -> Result<()> {
        let tpm = Tpm::new(config.socket).expect("Error creating TPM device");
        self.inner = Some(Arc::new(Mutex::new(tpm)));
        Ok(())
    }
    /// Get the inner TPM device
    pub fn get(&self) -> Option<&MutexTpm> {
        self.inner.as_ref()
    }
}
