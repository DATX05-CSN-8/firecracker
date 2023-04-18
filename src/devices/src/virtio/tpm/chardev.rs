use logger::error;
use tpm::chardev::CharDevTpm;

use super::device::TpmBackend;

impl TpmBackend for CharDevTpm {
    fn execute_command(&mut self, command: &[u8]) -> Vec<u8> {
        match self.exec(command) {
            Ok(resp) => resp,
            Err(err) => {
                error!("Error occurred delivering request to TPM emulator backend: {:?}", err);
                vec![]
            },
        }
    }
}