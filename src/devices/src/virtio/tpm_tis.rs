use std::fmt::{Display, Formatter};

use tpm::emulator::Emulator;

use crate::BusDevice;


/* Constants */
const TPM_TIS_NUM_LOCALITIES: u8 = 5;
const TPM_TIS_BUFFER_MAX: u32 = 4096;
const TPM_TIS_NO_LOCALITY: u8 = 255;
const TPM_TIS_ACCESS_TPM_REG_VALID_STS: u32 = 1 << 7;
const TPM_TIS_STS_TPM_FAMILY2_0: u32 = 1 << 26;
const TPM_TIS_IFACE_ID_SUPPORTED_FLAGS2_0: u32 = (0x0) | (0 << 4) | (1 << 8) | (1 << 13);
const TPM_TIS_INT_POLARITY_LOW_LEVEL: u32 = 1 << 3;
const TPM_TIS_ACCESS_SEIZE: u8 = 1 << 3;
const TPM_TIS_ACCESS_PENDING_REQUEST: u32 = 1 << 2;
const TPM_TIS_CAPABILITIES_SUPPORTED2_0: u32 = (1 << 4) | (0 << 8) | (3 << 9) | (3 << 28) | ((1 << 2) | (1 << 0) | (1 << 1) | (1 << 7));
const TPM_TIS_STS_DATA_AVAILABLE: u32 = 1 << 4;
const TPM_TIS_NO_DATA_BYTE: u32 = 0xff;
const TPM_TIS_TPM_DID: u32 = 0x0001;
const TPM_TIS_TPM_VID: u32 = 0x1014;
const TPM_TIS_TPM_RID: u32 = 0x0001;
const TPM_TIS_LOCALITY_SHIFT: u32 = 12;
const TPM_TIS_ACCESS_REQUEST_USE: u8 = 1 << 1;
const TPM_TIS_ACCESS_ACTIVE_LOCALITY: u8 = 1 << 5;
const TPM_TIS_ACCESS_BEEN_SEIZED: u32 = 1 << 4;
const TPM_TIS_INT_ENABLED: u32 = 1 << 31;
const TPM_TIS_INT_POLARITY_MASK: u32 = 3 << 3;
const TPM_TIS_INTERRUPTS_SUPPORTED: u32 = (1 << 2) | (1 << 0) | (1 << 1) | (1 << 7);
const TPM_TIS_STS_VALID: u32 = 1 << 7;
const TPM_TIS_INT_STS_VALID: u32 = 1 << 1;
const TPM_TIS_STS_SELFTEST_DONE: u32 = 1 << 2;
const TPM_TIS_STS_TPM_FAMILY_MASK: u32 = 0x3 << 26;
const TPM_TIS_STS_COMMAND_READY: u32 = 1 << 6;
const TPM_TIS_INT_DATA_AVAILABLE: u32 = 1 << 0;
const TPM_TIS_INT_LOCALITY_CHANGED: u32 = 1 << 2;
const TPM_TIS_INT_COMMAND_READY: u32 = 1 << 7;
const TPM_TIS_STS_COMMAND_CANCEL: u32 = 1 << 24;
const TPM_TIS_STS_RESET_ESTABLISHMENT_BIT: u32 = 1 << 25;
const TPM_TIS_STS_TPM_GO: u32 = 1 << 5;
const TPM_TIS_STS_RESPONSE_RETRY: u32 = 1 << 1;
const TPM_TIS_STS_EXPECT: u32 = 1 << 3;
const TPM_TIS_IFACE_ID_INT_SEL_LOCK: u32 = 1 << 19;

/* TIS registers */
const TPM_TIS_REG_ACCESS: u64 = 0x00;
const TPM_TIS_REG_INT_ENABLE: u64 = 0x08;
const TPM_TIS_REG_INT_VECTOR: u64 = 0x0c;
const TPM_TIS_REG_INT_STATUS: u64 = 0x10;
const TPM_TIS_REG_INTF_CAPABILITY: u64 = 0x14;
const TPM_TIS_REG_STS: u64 = 0x18;
const TPM_TIS_REG_DATA_FIFO: u64 = 0x24;
const TPM_TIS_REG_INTERFACE_ID: u64 = 0x30;
const TPM_TIS_REG_DATA_XFIFO: u64 = 0x80;
const TPM_TIS_REG_DATA_XFIFO_END:u64 = 0xbc;
const TPM_TIS_REG_DID_VID: u64 = 0xf00;
const TPM_TIS_REG_RID: u64 = 0xf04;

/* Helper Functions */
fn tpm_tis_locality_from_addr(addr: u64) -> u8 {
    ((addr >> TPM_TIS_LOCALITY_SHIFT) & 0x7) as u8
}


#[derive(Debug)]
pub enum TpmTisError { 
    TpmEmulatorError(String),
    TpmInitError(String),
}

impl Display for TpmTisError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use self::TpmTisError::*;
        match self {
            TpmEmulatorError(err) => write!(f, "Emulator doesn't implement min required capabilities: {:?}", err),
            TpmInitError(err) => write!(f, "Failed to initialize tpm: {:?}", err),
        }
    }
}

type Result<T> = std::result::Result<T, TpmTisError>;

/* TPM Device Structs */
#[derive(PartialEq, Debug)]
enum TPMTISState {
    TpmTisStateIdle,
    TpmTisStateReady,
    TpmTisStateCompletion,
    TpmTisStateExecution,
    TpmTisStateReception,
}

impl Clone for TPMTISState {
    fn clone(&self) -> Self {
        match self {
            TPMTISState::TpmTisStateIdle => TPMTISState::TpmTisStateIdle,
            TPMTISState::TpmTisStateReady => TPMTISState::TpmTisStateReady,
            TPMTISState::TpmTisStateCompletion => TPMTISState::TpmTisStateCompletion,
            TPMTISState::TpmTisStateExecution => TPMTISState::TpmTisStateExecution,
            TPMTISState::TpmTisStateReception => TPMTISState::TpmTisStateReception,
        }
    }
}

pub struct TpmTis {
    emulator: Emulator,
}

impl TpmTis {
    pub fn new(emulator: Emulator) -> Result<Self> {
        let tpmtis = Self {
            emulator,
        };
        // TODO reset
        Ok(tpmtis)
    }
}

impl BusDevice for TpmTis {
    fn read(&mut self, offset: u64, data: &mut [u8]) {
        // TODO
    }
    fn write(&mut self, offset: u64, data: &[u8]) {
        // TODO
    }
}