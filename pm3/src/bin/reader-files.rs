//! Bounded, read-only software backup through the installed reader's GetFile IPP.
//! No card polling, card commands, writes, resets or arbitrary paths are exposed.
#[path = "../ipp.rs"]
mod ipp;
#[path = "../reader.rs"]
mod reader;

use std::{
    fs::OpenOptions,
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, Instant},
};

const FILES: &[(&str, &str)] = &[
    ("/home/app0/version", "app-version"),
    ("/home/app0/run", "app-run"),
    ("/home/startup_config.conf", "startup-config"),
    ("/home/app0/feig", "feig"),
    ("/home/app0/feig_app_loader", "feig_app_loader"),
    ("/usr/lib/libfeclr.so", "libfeclr.so"),
    ("/usr/lib/libfetpf.so", "libfetpf.so"),
];

fn read_chunk(reader: &mut reader::Reader, path: &str, offset: u32) -> io::Result<Vec<u8>> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&(path.len() as u16).to_be_bytes());
    payload.extend_from_slice(path.as_bytes());
    payload.extend_from_slice(&offset.to_be_bytes());
    payload.extend_from_slice(&4096u16.to_be_bytes());
    reader.send(0x98, &payload)?;
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        for message in reader.poll()? {
            if message.kind != 0x99 {
                continue;
            }
            if message.payload.len() < 4 || message.payload[..2] != [0, 0] {
                return Err(io::Error::other("reader refused software-file read"));
            }
            let length = u16::from_be_bytes([message.payload[2], message.payload[3]]) as usize;
            if length > 4096 || message.payload.len() != length + 4 {
                return Err(io::Error::other("invalid file-reply length"));
            }
            return Ok(message.payload[4..].to_vec());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "reader file reply timed out",
    ))
}

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err(io::Error::other("Usage: reader-files OUTPUT_DIRECTORY"));
    }
    let output = Path::new(&args[1]);
    std::fs::create_dir(output)?; // Refuse overwriting any prior backup.
    let mut reader = reader::Reader::open("/dev/ttymxc3")?;
    let overall = Instant::now() + Duration::from_secs(600);
    for (path, name) in FILES {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(output.join(name))?;
        let mut offset = 0u32;
        loop {
            if Instant::now() > overall || offset >= 8 * 1024 * 1024 {
                return Err(io::Error::other("software backup limit reached"));
            }
            let chunk = read_chunk(&mut reader, path, offset)?;
            file.write_all(&chunk)?;
            offset += chunk.len() as u32;
            if chunk.len() < 4096 {
                break;
            }
        }
        file.sync_all()?;
        eprintln!("Saved {name}: {offset} bytes");
    }
    Ok(())
}
