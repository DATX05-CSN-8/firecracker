// Copyright © 2022, Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use std::io::Read;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use vmm_sys_util::sock_ctrl_msg::ScmSocket;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum TpmSocketError { 
    ConnectToSocket(String),
    ReadFromSocket(String),
    WriteToSocket(String),
}
impl Display for TpmSocketError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use self::TpmSocketError::*;
        match self {
            ConnectToSocket(err) => write!(f, "Cannot connect to tpm Socket: {:?}", err),
            ReadFromSocket(err) => write!(f, "Failed to read from socket: {:?}", err),
            WriteToSocket(err) => write!(f, "Failed to write to socket: {:?}", err),
        }
    }
}

type Result<T> = std::result::Result<T, TpmSocketError>;

#[derive(PartialEq)]
enum SocketDevState {
    Disconnected,
    Connecting,
    Connected,
}

pub struct SocketDev {
    state: SocketDevState,
    stream: Option<UnixStream>,
    // Fd sent to swtpm process for Data Channel
    write_msgfd: RawFd,
    // Data Channel used by Cloud-Hypervisor
    data_fd: RawFd,
    // Control Channel used by Cloud-Hypervisor
    control_fd: RawFd,
}

impl Default for SocketDev {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketDev {
    pub fn new() -> Self {
        Self {
            state: SocketDevState::Disconnected,
            stream: None,
            write_msgfd: -1,
            control_fd: -1,
            data_fd: -1,
        }
    }

    pub fn init(&mut self, path: String) -> Result<()> {
        self.connect(&path)?;
        Ok(())
    }

    pub fn connect(&mut self, socket_path: &str) -> Result<()> {
        self.state = SocketDevState::Connecting;

        let s = UnixStream::connect(socket_path).map_err(|e| TpmSocketError::ConnectToSocket(
            format!("{} {:?} ","Failed to connect to tpm Socket. Error:", e)))?;
        self.control_fd = s.as_raw_fd();
        self.stream = Some(s);
        self.state = SocketDevState::Connected;
        debug!("Connected to tpm socket path : {:?}", socket_path);
        Ok(())
    }

    pub fn set_datafd(&mut self, fd: RawFd) {
        self.data_fd = fd;
    }

    pub fn set_msgfd(&mut self, fd: RawFd) {
        self.write_msgfd = fd;
    }

    pub fn send_full(&self, buf: &[u8]) -> Result<usize> {
        let write_fd = self.write_msgfd;

        let size = self
            .stream
            .as_ref()
            .unwrap()
            .send_with_fd(buf, write_fd)
            .map_err(|e| TpmSocketError::WriteToSocket(
                format!("{} {:?} ","Failed to write to Socket. Error:", e)))?;

        Ok(size)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.stream.is_none() {
            return Err(TpmSocketError::WriteToSocket(String::from("TPM Socket was not in Connected State")));
        }

        if matches!(self.state, SocketDevState::Connected) {
            let ret = self.send_full(buf)?;
            // swtpm will receive data Fd after a successful send
            // Reset cached write_msgfd after a successful send
            // Ideally, write_msgfd is reset after first Ctrl Command
            if ret > 0 && self.write_msgfd != 0 {
                self.write_msgfd = 0;
            }
            Ok(ret)
        } else {
            Err(TpmSocketError::WriteToSocket(String::from("TPM Socket was not in Connected State")))
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.stream.is_none() {
            return Err(TpmSocketError::ReadFromSocket(String::from("Stream for tpm socket was not initialized")));
        }
        let mut socket = self.stream.as_ref().unwrap();
        let size: usize = socket.read(buf).map_err(|e| TpmSocketError::ReadFromSocket(
            format!("{} {:?} ","Failed to read from socket. Error Code", e)))?;
        Ok(size)
    }
}
