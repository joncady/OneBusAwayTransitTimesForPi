//! Explicit, allowlisted staging for the temporary FeliCa reader application trial.
//! Writes only trial-owned files and the backed-up app0 run script. No card writes.
#[path = "../ipp.rs"]
mod ipp;
#[path = "../reader.rs"]
mod reader;
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::{Duration, Instant},
};
const BASE: &str = "/home/app0/";
fn exchange(r: &mut reader::Reader, kind: u8, payload: &[u8], reply: u8) -> io::Result<Vec<u8>> {
    r.send(kind, payload)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        for m in r.poll()? {
            if m.kind == reply {
                return Ok(m.payload);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(io::Error::other("reader maintenance reply timeout"))
}
fn header(name: &str, offset: usize, length: usize) -> Vec<u8> {
    let path = format!("{BASE}{name}");
    let mut bytes = (path.len() as u16).to_be_bytes().to_vec();
    bytes.extend_from_slice(path.as_bytes());
    bytes.extend_from_slice(&(offset as u32).to_be_bytes());
    bytes.extend_from_slice(&(length as u16).to_be_bytes());
    bytes
}
fn get(r: &mut reader::Reader, name: &str) -> io::Result<Vec<u8>> {
    let mut data = vec![];
    loop {
        if data.len() > 128 * 1024 {
            return Err(io::Error::other("trial file too large"));
        }
        let p = exchange(r, 0x98, &header(name, data.len(), 4096), 0x99)?;
        if p.len() < 4 || p[..2] != [0, 0] {
            return Err(io::Error::other("reader refused trial-file read"));
        }
        let n = u16::from_be_bytes([p[2], p[3]]) as usize;
        if n > 4096 || p.len() != n + 4 {
            return Err(io::Error::other("invalid trial-file length"));
        }
        data.extend_from_slice(&p[4..]);
        if n < 4096 {
            return Ok(data);
        }
    }
}
fn put(r: &mut reader::Reader, name: &str, data: &[u8]) -> io::Result<()> {
    if data.is_empty() || data.len() > 128 * 1024 {
        return Err(io::Error::other("invalid trial file"));
    }
    if get(r, name).is_ok_and(|existing| existing == data) {
        eprintln!("Unchanged {name}: {} bytes", data.len());
        return Ok(());
    }
    for (i, chunk) in data.chunks(2048).enumerate() {
        let mut p = header(name, i * 2048, chunk.len());
        p.extend_from_slice(chunk);
        if exchange(r, 0x96, &p, 0x97)? != [0, 0] {
            return Err(io::Error::other("reader refused trial-file write"));
        }
    }
    if get(r, name)? != data {
        return Err(io::Error::other("trial-file readback mismatch"));
    }
    eprintln!("Verified {name}: {} bytes", data.len());
    Ok(())
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err(io::Error::other(
            "Usage: reader-trial stage|restore|fetch DIRECTORY, or reboot DIRECTORY",
        ));
    }
    let dir = Path::new(&args[2]);
    let mut r = reader::Reader::open("/dev/ttymxc3")?;
    match args[1].as_str() {
        "stage" => {
            let original = fs::read(dir.join("original-run"))?;
            if get(&mut r, "version")? != b"02.35\n" || get(&mut r, "run")? != original {
                return Err(io::Error::other("reader version/run does not match backup"));
            }
            put(&mut r, "pm3-original-run", &original)?;
            // Canary and readback happen before any live launch configuration changes.
            put(&mut r, "pm3-trial-canary", b"pm3-felica-trial-v1\n")?;
            if dir.join("mapping-probe").exists() {
                put(
                    &mut r,
                    "pm3-mapping-probe",
                    &fs::read(dir.join("mapping-probe"))?,
                )?;
            }
            if dir.join("rf-probe").exists() {
                put(&mut r, "pm3-rf-probe", &fs::read(dir.join("rf-probe"))?)?;
            }
            put(&mut r, "pm3-adapter.so", &fs::read(dir.join("adapter.so"))?)?;
            put(&mut r, "pm3-once.sh", &fs::read(dir.join("once.sh"))?)?;
            let launcher = fs::read(dir.join("launcher"))?;
            if launcher != b"#!/bin/sh\nexec /bin/sh /home/app0/pm3-once.sh\n" {
                return Err(io::Error::other("unexpected launcher"));
            }
            put(&mut r, "run", &launcher)?;
        }
        "restore" => {
            put(&mut r, "run", &fs::read(dir.join("original-run"))?)?;
        }
        "fetch" => {
            fs::create_dir_all(dir)?;
            for name in ["run", "pm3-trial-status", "pm3-trial-output"] {
                let bytes = get(&mut r, name)?;
                let mut f = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(dir.join(name))?;
                f.write_all(&bytes)?;
                eprintln!("Saved {name}: {} bytes", bytes.len());
            }
        }
        "reboot" => {
            let reply = exchange(&mut r, 0x10, &[3], 0x11)?;
            if reply != [0, 0] {
                return Err(io::Error::other("reader reboot rejected"));
            }
            eprintln!("Reader module reboot acknowledged");
        }
        _ => return Err(io::Error::other("unknown trial action")),
    }
    Ok(())
}
