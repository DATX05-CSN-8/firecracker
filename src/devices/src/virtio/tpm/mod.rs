pub mod device;
pub mod queue_utils;
pub mod event_handler;
pub mod emulator;

use std::io;

use thiserror::Error;
use vm_memory::{GuestMemoryError};

pub use self::device::Tpm;
pub use self::event_handler::*;


// Maximum command or response message size permitted by this device
// implementation. Named to match the equivalent constant in Linux's tpm.h.
// There is no hard requirement that the value is the same but it makes sense.
const TPM_BUFSIZE: usize = 4096;

pub const TPM_DEV_ID: &str = "vtpm";

#[derive(Error, Debug)]
pub enum TpmError {
    #[error("vtpm response buffer is too small: {size} < {required} bytes")]
    BufferTooSmall { size: usize, required: usize },
    #[error("vtpm command is too long: {size} > {} bytes", TPM_BUFSIZE)]
    CommandTooLong { size: usize },
    #[error("vtpm eventfd error: {0}")]
    EventFd(io::Error),
    #[error("vtpm irqtrigger error: {0}")]
    IrqTrigger(io::Error),
    #[error(
        "vtpm simulator generated a response that is unexpectedly long: {size} > {} bytes",
        TPM_BUFSIZE
    )]
    ResponseTooLong { size: usize },
    #[error("vtpm failed accessing guest memory: {0}")]
    GuestMemory(GuestMemoryError),

}
