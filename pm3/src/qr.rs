//! Read-only QR input. No scanner power/trigger commands or arbitrary URL actions.
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    mem::MaybeUninit,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    time::{Duration, Instant},
};

const PREFIX: &[u8] = b"PM3-ALBUM:1:";
const MAX_RECORD: usize = 256;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    version: u8,
    enabled: bool,
    device: String,
    cooldown_ms: u64,
    frame_gap_ms: u64,
    reconnect_seconds: u64,
    // Keys are opaque lowercase hex tokens; values refer to existing media albums.
    #[serde(deserialize_with = "unique_albums")]
    albums: BTreeMap<String, String>,
}

fn unique_albums<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Albums;
    impl<'de> serde::de::Visitor<'de> for Albums {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("unique QR token mappings")
        }
        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut albums = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if albums.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate QR token"));
                }
            }
            Ok(albums)
        }
    }
    deserializer.deserialize_map(Albums)
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if fs::metadata(path)?.len() > 16 * 1024 {
            return Err("QR configuration exceeds 16 KiB".into());
        }
        let config: Self = serde_json::from_slice(&fs::read(path)?)?;
        let device_valid = config
            .device
            .strip_prefix("/dev/ttyUSB")
            .is_some_and(|index| {
                index
                    .parse::<u8>()
                    .is_ok_and(|n| n <= 31 && n.to_string() == index)
            });
        if config.version != 1
            || !device_valid
            || !(250..=10000).contains(&config.cooldown_ms)
            || !(50..=1000).contains(&config.frame_gap_ms)
            || !(2..=60).contains(&config.reconnect_seconds)
            || config.albums.is_empty()
            || config.albums.len() > 32
            || config.albums.iter().any(|(token, label)| {
                !valid_token(token.as_bytes())
                    || label.trim().is_empty()
                    || label.chars().count() > 32
                    || label.chars().any(char::is_control)
            })
        {
            return Err("invalid QR configuration".into());
        }
        Ok(config)
    }

    pub fn album_labels(&self) -> impl Iterator<Item = &str> {
        self.albums.values().map(String::as_str)
    }

    pub fn start(mut self, available: impl Fn(&str) -> bool) -> Option<Scanner> {
        if !self.enabled {
            return None;
        }
        let before = self.albums.len();
        self.albums.retain(|_, label| available(label));
        if self.albums.len() != before {
            eprintln!("QR input skipped unavailable album mappings");
        }
        if self.albums.is_empty() {
            eprintln!("QR input disabled: no usable mapped albums");
            return None;
        }
        Some(Scanner {
            config: self,
            port: None,
            retry_at: Instant::now(),
            buffer: Vec::with_capacity(MAX_RECORD),
            discarding: false,
            last_byte: None,
            last_trigger: None,
            reported_unavailable: false,
        })
    }
}

fn valid_token(token: &[u8]) -> bool {
    token.len() == 32
        && token
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
}

struct Port {
    file: File,
    original: libc::termios,
}

impl Port {
    fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)?;
        let fd = file.as_raw_fd();
        if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut old = MaybeUninit::<libc::termios>::uninit();
        if unsafe { libc::tcgetattr(fd, old.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let original = unsafe { old.assume_init() };
        let port = Self { file, original };
        let mut settings = original;
        unsafe {
            libc::cfmakeraw(&mut settings);
            libc::cfsetispeed(&mut settings, libc::B115200);
            libc::cfsetospeed(&mut settings, libc::B115200);
        }
        settings.c_cflag |= libc::CLOCAL | libc::CREAD;
        settings.c_cflag &= !(libc::CSIZE | libc::CSTOPB | libc::PARENB | libc::CRTSCTS);
        settings.c_cflag |= libc::CS8;
        settings.c_cc[libc::VMIN] = 0;
        settings.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &settings) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Discard stale scans received before this process opened the port.
        if unsafe { libc::tcflush(fd, libc::TCIFLUSH) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(port)
    }

    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let mut pfd = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut pfd, 1, 0) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        if pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            return Err(io::Error::other("QR serial disconnected"));
        }
        if pfd.revents & libc::POLLIN == 0 {
            return Ok(0);
        }
        self.file.read(bytes)
    }
}

impl Drop for Port {
    fn drop(&mut self) {
        if unsafe { libc::tcsetattr(self.file.as_raw_fd(), libc::TCSANOW, &self.original) } != 0 {
            eprintln!("QR UART restore incomplete; next open will configure the port explicitly");
        }
    }
}

pub struct Scanner {
    config: Config,
    port: Option<Port>,
    retry_at: Instant,
    buffer: Vec<u8>,
    discarding: bool,
    last_byte: Option<Instant>,
    last_trigger: Option<Instant>,
    reported_unavailable: bool,
}

impl Scanner {
    fn clear_record(&mut self) {
        self.buffer.clear();
        self.discarding = false;
        self.last_byte = None;
    }

    fn finish_record(&mut self) -> Option<String> {
        let label = if !self.discarding
            && self.buffer.starts_with(PREFIX)
            && valid_token(&self.buffer[PREFIX.len()..])
        {
            let token = std::str::from_utf8(&self.buffer[PREFIX.len()..]).ok()?;
            self.config.albums.get(token).cloned()
        } else {
            None
        };
        self.clear_record();
        label
    }

    /// At most 2048 input bytes per UI iteration; no blocking waits or writes.
    pub fn poll(&mut self) -> Option<String> {
        if self.port.is_none() {
            if Instant::now() < self.retry_at {
                return None;
            }
            match Port::open(&self.config.device) {
                Ok(port) => {
                    self.port = Some(port);
                    self.reported_unavailable = false;
                    self.clear_record();
                    println!("QR serial connected (read-only, 115200 8N1)");
                }
                Err(_) => {
                    self.retry_at =
                        Instant::now() + Duration::from_secs(self.config.reconnect_seconds);
                    if !self.reported_unavailable {
                        eprintln!("QR serial unavailable; retry scheduled");
                        self.reported_unavailable = true;
                    }
                    return None;
                }
            }
        }
        let mut candidate = None;
        let mut bytes = [0u8; 256];
        let mut drained = false;
        for _ in 0..8 {
            match self.port.as_mut().unwrap().read(&mut bytes) {
                Ok(0) => {
                    drained = true;
                    break;
                }
                Ok(count) => {
                    for byte in &bytes[..count] {
                        if matches!(byte, b'\r' | b'\n') {
                            let label = self.finish_record();
                            if candidate.is_none() {
                                candidate = label;
                            }
                        } else {
                            self.last_byte = Some(Instant::now());
                            if !self.discarding {
                                if self.buffer.len() == MAX_RECORD {
                                    self.buffer.clear();
                                    self.discarding = true;
                                } else {
                                    self.buffer.push(*byte);
                                }
                            }
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    drained = true;
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => break,
                Err(_) => {
                    self.port = None;
                    self.clear_record();
                    self.retry_at =
                        Instant::now() + Duration::from_secs(self.config.reconnect_seconds);
                    eprintln!("QR serial disconnected; retry scheduled");
                    self.reported_unavailable = true;
                    return None;
                }
            }
        }
        // The installed scanner emits no CRLF. A quiet gap completes one scan.
        // Oversized/malformed/concatenated records are rejected as a whole.
        if drained
            && self.last_byte.is_some_and(|last| {
                last.elapsed() >= Duration::from_millis(self.config.frame_gap_ms)
            })
        {
            let label = self.finish_record();
            if candidate.is_none() {
                candidate = label;
            }
        }
        if self
            .last_trigger
            .is_some_and(|last| last.elapsed() < Duration::from_millis(self.config.cooldown_ms))
        {
            return None;
        }
        if candidate.is_some() {
            self.last_trigger = Some(Instant::now());
        }
        candidate
    }
}
