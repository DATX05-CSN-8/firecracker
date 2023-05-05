use logger::error;
use tpm::emulator::{Emulator, BackendCmd};

use crate::virtio::tpm::TPM_BUFSIZE;

use super::device::TpmBackend;

impl TpmBackend for Emulator {
    fn execute_command<'a>(&'a mut self, command: &[u8]) -> Vec<u8> {
        let mut buf = [0u8; TPM_BUFSIZE];
        buf[..command.len()].copy_from_slice(command);
        let mut mapped = BackendCmd {
            buffer: &mut buf,
            input_len: command.len()
        };
        match self.deliver_request(&mut mapped) {
            Ok(size) => mapped.buffer[..size].to_vec(),
            Err(err) => {
                error!("Error occurred delivering request to TPM emulator backend: {:?}", err);
                return vec![];
            }
        }
    }
}