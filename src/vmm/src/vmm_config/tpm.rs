
use std::fmt;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use devices::virtio::tpm::{Tpm, TpmError};
use tpm::emulator::{Emulator, TpmEmulatorError};

type MutexTpm = Arc<Mutex<Tpm>>;

/// Errors associated with TPM config errors
#[derive(Debug, derive_more::From)]
pub enum TpmConfigError {
    /// General TPM config error, TODO change 
    CreateTpmVirtioDevice(TpmError),
    /// Cannot create tpm device
    CreateTpmEmulator(TpmEmulatorError), // TODO AAA kolla vsock.rs VsockError
    /// Missing path for TPM device
    ParseTpmPathMissing,
}

impl fmt::Display for TpmConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TpmConfigError::CreateTpmVirtioDevice(err) => write!(f, "Failed to create TPM virtio device: {:?}", err),
            TpmConfigError::CreateTpmEmulator(err) => write!(f, "Failed to create TPM Emulator: {:?}", err),
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
        // TODO verify path to socket
        let emulator = match Emulator::new(config.socket) {
            Ok(emu) => emu,
            Err(err) => {
                return Err(TpmConfigError::CreateTpmEmulator(err))
            }
        };
        match Tpm::new(Box::new(emulator)) {
            Ok(tpm) => {
                self.inner = Some(Arc::new(Mutex::new(tpm)));
                Ok(())
            },
            Err(err) => Err(TpmConfigError::CreateTpmVirtioDevice(err))
        }
    }
    
    /// Get the inner TPM device
    pub fn get(&self) -> Option<&MutexTpm> {
        self.inner.as_ref()
    }
}
