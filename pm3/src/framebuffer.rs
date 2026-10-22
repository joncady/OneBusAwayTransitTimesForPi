use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
};

pub struct Framebuffer {
    file: File,
    original: Vec<u8>,
    pub width: usize,
    stride: usize,
    output: Vec<u8>,
    _cursor: CursorBlink,
}

// The framebuffer console otherwise blinks a cursor over the native display.
// Preserve its setting and pause it only for the lifetime of the prototype.
struct CursorBlink(Option<String>);
impl CursorBlink {
    const PATH: &str = "/sys/class/graphics/fbcon/cursor_blink";
    fn pause() -> Self {
        let original = std::fs::read_to_string(Self::PATH).ok();
        if original.is_some() && std::fs::write(Self::PATH, "0").is_ok() {
            Self(original)
        } else {
            Self(None)
        }
    }
}
impl Drop for CursorBlink {
    fn drop(&mut self) {
        if let Some(original) = &self.0 {
            let _ = std::fs::write(Self::PATH, original);
        }
    }
}

impl Framebuffer {
    pub fn open(path: &str) -> io::Result<Self> {
        // Validate this known hardware before writing any pixels. Do not change modes.
        if path != "/dev/fb0" {
            return Err(io::Error::other(
                "prototype only supports the audited /dev/fb0",
            ));
        }
        let size = std::fs::read_to_string("/sys/class/graphics/fb0/virtual_size")?;
        let bits = std::fs::read_to_string("/sys/class/graphics/fb0/bits_per_pixel")?;
        let stride: usize = std::fs::read_to_string("/sys/class/graphics/fb0/stride")?
            .trim()
            .parse()
            .map_err(io::Error::other)?;
        if size.trim() != "800,480" || bits.trim() != "32" || stride != 3200 {
            return Err(io::Error::other(
                "unexpected framebuffer geometry; refusing to write",
            ));
        }
        let cursor = CursorBlink::pause();
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        let mut original = vec![0; stride * 480];
        file.read_exact(&mut original)?;
        Ok(Self {
            file,
            output: vec![0; original.len()],
            original,
            width: 800,
            stride,
            _cursor: cursor,
        })
    }

    pub fn draw(&mut self, pixels: &[slint::Rgb8Pixel]) -> io::Result<()> {
        for (y, row) in pixels.chunks_exact(self.width).enumerate() {
            for (x, pixel) in row.iter().enumerate() {
                let offset = y * self.stride + x * 4;
                self.output[offset..offset + 4].copy_from_slice(&[pixel.b, pixel.g, pixel.r, 255]);
            }
        }
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&self.output)
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        let restored = (|| -> io::Result<bool> {
            self.file.seek(SeekFrom::Start(0))?;
            self.file.write_all(&self.original)?;
            self.file.seek(SeekFrom::Start(0))?;
            self.file.read_exact(&mut self.output)?;
            Ok(self.output == self.original)
        })();
        match restored {
            Ok(true) => println!("Framebuffer restore verified by reading back original pixels"),
            Ok(false) => eprintln!("Framebuffer restore readback differed from saved pixels"),
            Err(error) => eprintln!("Framebuffer restore could not be verified: {error}"),
        }
    }
}
