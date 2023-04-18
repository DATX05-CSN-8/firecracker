use std::{io::{Read, Write}, fs::{OpenOptions, File}};

#[derive(Debug)]
pub enum CharDevTpmError {
    ExecError,
    InitError(std::io::Error)
}

type Result<T> = std::result::Result<T, CharDevTpmError>;

pub struct CharDevTpm {
    rsp: Vec<u8>,
    file: File
}

fn open(path: &str) -> std::io::Result<File> {
    OpenOptions::new().read(true).write(true).open(path)
}

impl CharDevTpm {
    pub fn new(path: String) -> Result<Self> {
        let f = match open(&path) {
            Ok(f) => f,
            Err(e) => return Err(CharDevTpmError::InitError(e))
        };
        Ok(CharDevTpm {
            rsp: vec![],
            file: f,
        })
    }

    pub fn exec(&mut self, cmd: &[u8]) -> std::result::Result<Vec<u8>, std::io::Error> {
        self.file.write_all(cmd)?;
        self.rsp.clear();
        self.file.read_to_end(&mut self.rsp)?;
        Ok(self.rsp.to_vec())
    } 
}