//! Explicit opt-in capture of discovery-only FeliCa diagnostic messages.
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
};
pub struct Capture {
    file: File,
    count: usize,
    pub ready: bool,
}
impl Capture {
    pub fn open(path: &str) -> io::Result<Self> {
        Ok(Self {
            file: OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)?,
            count: 0,
            ready: false,
        })
    }
    pub fn event(&mut self, payload: &[u8]) -> io::Result<Option<usize>> {
        if payload == [1, 0] {
            self.ready = true;
            println!("FeliCa trial adapter ready");
            return Ok(None);
        }
        if !self.ready || payload.len() != 514 || payload[..2] != [1, 4] || self.count >= 64 {
            return Ok(None);
        }
        self.file.write_all(payload)?;
        self.file.sync_data()?;
        self.count += 1;
        println!("FeliCa discovery captured: scan={}", self.count);
        Ok(Some(self.count))
    }
}
