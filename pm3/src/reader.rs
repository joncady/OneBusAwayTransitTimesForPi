use crate::ipp::{self, Decoder, Message};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    mem::MaybeUninit,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
};

pub struct Reader {
    port: File,
    original: libc::termios,
    sequence: u8,
    detection_enabled: bool,
    leds_used: bool,
    pub decoder: Decoder,
}

impl Reader {
    pub fn open(path: &str) -> io::Result<Self> {
        let port = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOCTTY | libc::O_NONBLOCK)
            .open(path)?;
        let fd = port.as_raw_fd();
        // Advisory lock prevents two copies of this prototype sharing the port.
        if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut old = MaybeUninit::<libc::termios>::uninit();
        if unsafe { libc::tcgetattr(fd, old.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let original = unsafe { old.assume_init() };
        let mut settings = original;
        unsafe {
            libc::cfmakeraw(&mut settings);
            libc::cfsetispeed(&mut settings, libc::B115200);
            libc::cfsetospeed(&mut settings, libc::B115200);
        }
        settings.c_cflag |= libc::CLOCAL | libc::CREAD;
        settings.c_cflag &= !libc::CRTSCTS;
        settings.c_cc[libc::VMIN] = 0;
        settings.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &settings) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            port,
            original,
            sequence: 0,
            detection_enabled: false,
            leds_used: false,
            decoder: Decoder::default(),
        })
    }

    /// Documented host-side indicator bitmap; does not access card contents.
    pub fn set_leds(&mut self, mask: u32) -> io::Result<()> {
        self.leds_used = true;
        self.send(0x20, &mask.to_be_bytes())
    }

    /// cVEND's own buzzer; independent of Linux PCM playback and its amplifier.
    pub fn beep(&mut self, frequency: u16, duration_ms: u16) -> io::Result<()> {
        self.send(0x22, &buzzer_payload(frequency, duration_ms)?)
    }

    pub fn enable_detection(&mut self) -> io::Result<()> {
        self.detection_enabled = true;
        // Release any session left by a previous process before re-enabling RF polling.
        self.send(0x32, &[])?;
        self.send(0xe4, &[0, 6, 1, 1])?;
        self.send(0xe4, &[0, 7, 1, 1])
    }

    pub fn send(&mut self, kind: u8, payload: &[u8]) -> io::Result<()> {
        self.sequence = self.sequence.wrapping_add(1).max(1);
        let bytes = ipp::frame(self.sequence, kind, payload);
        // These commands are only a few bytes. Bound waiting for a stalled UART.
        let mut offset = 0;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
        while offset < bytes.len() {
            match self.port.write(&bytes[offset..]) {
                Ok(0) => return Err(io::Error::new(io::ErrorKind::WriteZero, "NFC UART stalled")),
                Ok(n) => offset += n,
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn poll(&mut self) -> io::Result<Vec<Message>> {
        let mut bytes = [0u8; 4096];
        let mut messages = vec![];
        // Bound each iteration so continuous reader output cannot starve the UI.
        for _ in 0..16 {
            match self.port.read(&mut bytes) {
                Ok(0) => break,
                Ok(n) => messages.extend(self.decoder.feed(&bytes[..n])),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        Ok(messages)
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        if self.leds_used {
            let _ = self.set_leds(0);
        }
        if self.detection_enabled {
            let _ = self.send(0x32, &[]);
            let _ = self.send(0xe4, &[0, 6, 1, 0]);
            let _ = self.send(0xe4, &[0, 7, 1, 0]);
        }
        unsafe {
            libc::tcsetattr(self.port.as_raw_fd(), libc::TCSANOW, &self.original);
        }
    }
}

fn buzzer_payload(frequency: u16, duration_ms: u16) -> io::Result<[u8; 4]> {
    if !(100..=5000).contains(&frequency) || !(20..=1000).contains(&duration_ms) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "buzzer frequency must be 100..5000 Hz and duration 20..1000 ms",
        ));
    }
    let frequency = frequency.to_be_bytes();
    let duration = duration_ms.to_be_bytes();
    Ok([frequency[0], frequency[1], duration[0], duration[1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_buzzer_payload_and_frame() {
        let payload = buzzer_payload(1536, 256).unwrap();
        assert_eq!(payload, [6, 0, 1, 0]);
        let frame = ipp::frame(1, 0x22, &payload);
        assert_eq!(frame.len(), 11); // Short command: no payload CRC32.
        let mut decoder = Decoder::default();
        assert_eq!(
            decoder.feed(&frame),
            vec![Message {
                kind: 0x22,
                payload: payload.to_vec()
            }]
        );
    }

    #[test]
    fn buzzer_parameters_are_bounded() {
        for (hz, ms) in [(99, 256), (5001, 256), (1536, 19), (1536, 1001)] {
            assert!(buzzer_payload(hz, ms).is_err());
        }
        assert!(buzzer_payload(100, 20).is_ok());
        assert!(buzzer_payload(5000, 1000).is_ok());
    }
}
