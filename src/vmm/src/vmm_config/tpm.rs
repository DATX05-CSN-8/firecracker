
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::device_manager::mmio::MMIODeviceManager;
use devices::virtio::tpm::Tpm;

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
        match *self {
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
    inner: Option<TpmDeviceConfig>,
}

impl TpmBuilder {
    pub fn set(&mut self, tpm_path: TpmDeviceConfig) -> Result<()> {

        // Create TPM Device
        let tpm = Tpm::new(tpm_path.socket.clone()).map_err(|err| TpmConfigError::CreateTpmDevice(err.to_string()))?;

        // Add TPM Device to mmio
        self.inner = Some(TpmDeviceConfig{ socket: MMIODeviceManager::register_tpm(&mut MMIODeviceManager, tpm)}); //TODO AAA suggested the &mut mmio::MM..
        Ok(())
    }

}
