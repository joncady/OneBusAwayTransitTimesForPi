//! Bounded ARMv5 discovery-only trial using the exact installed libfeclr ioctl ABI.
//! No APDUs, authentication, fare operations, card writes, keys or scanner access.
#![no_std]
#![no_main]
use core::arch::global_asm;
#[cfg(all(fetpf_client, not(loader_probe)))]
include!("service-client.rs");
#[cfg(loader_probe)]
include!("loader-probe.rs");
global_asm!(
    r#"
    .syntax unified
    .arm
    .global call6
call6:
    push {{r4,r5,r7,lr}}
    mov r7,r0
    mov r0,r1
    mov r1,r2
    mov r2,r3
    ldr r3,[sp,#16]
    ldr r4,[sp,#20]
    ldr r5,[sp,#24]
    svc #0
    pop {{r4,r5,r7,pc}}
"#
);
#[cfg(not(fetpf_client))]
global_asm!(".global _start\n_start:\n bl probe\n b .");
#[cfg(fetpf_client)]
global_asm!(
    r#"
    .global _start
_start:
    mov r4, r0
    mov r3, sp
    ldr r1, [sp]
    add r2, sp, #4
    sub sp, sp, #16
    str r3, [sp, #8]
    str r4, [sp, #4]
    mov r3, #0
    str r3, [sp]
    ldr r0, =service_main
    bl __libc_start_main
    b .
"#
);
unsafe extern "C" {
    fn call6(n: i32, a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32;
}
// Freestanding equivalents used by LLVM for the bounded stack buffers.
#[unsafe(no_mangle)]
#[cfg(not(fetpf_client))]
unsafe extern "C" fn memset(dst: *mut u8, value: i32, len: usize) -> *mut u8 {
    for i in 0..len {
        unsafe {
            dst.add(i).write_volatile(value as u8);
        }
    }
    dst
}
#[unsafe(no_mangle)]
#[cfg(not(fetpf_client))]
unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    for i in 0..len {
        unsafe {
            dst.add(i).write_volatile(src.add(i).read_volatile());
        }
    }
    dst
}
fn syscall(n: i32, a: i32, b: i32, c: i32) -> i32 {
    unsafe { call6(n, a, b, c, 0, 0, 0) }
}
fn note(s: &[u8]) {
    syscall(4, 1, s.as_ptr() as i32, s.len() as i32);
}
fn number(mut n: i32) {
    if n < 0 {
        note(b"-");
        n = -n;
    }
    let mut b = [0u8; 12];
    let mut i = 12;
    loop {
        i -= 1;
        b[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    note(&b[i..]);
    note(b"\n");
}
fn exit(n: i32) -> ! {
    syscall(1, n, 0, 0);
    loop {
        core::hint::spin_loop();
    }
}
fn now() -> u32 {
    let mut t = [0i32; 2];
    syscall(263, 1, t.as_mut_ptr() as i32, 0);
    t[0] as u32
}
fn sleep() {
    let t = [0i32, 10_000_000];
    syscall(162, t.as_ptr() as i32, 0, 0);
}
fn ioctl(fd: i32, request: u32, data: *mut u8) -> i32 {
    syscall(54, fd, request as i32, data as i32)
}
fn crc8(bytes: &[u8]) -> u8 {
    let mut c = 0u8;
    for b in bytes {
        c ^= b;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0x8c } else { 0 };
        }
    }
    c
}
fn send(fd: i32, sequence: &mut u8, payload: &[u8]) {
    let mut b = [0u8; 525];
    *sequence = sequence.wrapping_add(1).max(1);
    b[0] = 0xbc;
    b[1] = *sequence;
    b[3] = 0xf1;
    b[4] = (payload.len() >> 8) as u8;
    b[5] = payload.len() as u8;
    b[6] = crc8(&b[..6]);
    let mut crc = u32::MAX;
    for (i, v) in payload.iter().enumerate() {
        b[7 + i] = *v;
        crc ^= u32::from(*v);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    let end = 7 + payload.len();
    b[end..end + 4].copy_from_slice(&crc.to_le_bytes());
    let mut offset = 0;
    let started = now();
    while offset < end + 4 && now().wrapping_sub(started) < 2 {
        let n = syscall(
            4,
            fd,
            b[offset..].as_ptr() as i32,
            (end + 4 - offset) as i32,
        );
        if n > 0 {
            offset += n as usize;
        } else if n == -11 || n == -4 {
            sleep();
        } else {
            break;
        }
    }
}
fn start(fd: i32) -> bool {
    // libfeclr start: loop 1, RF technology 4 (FeliCa), parameter 1, output status.
    let mut p = [1u64, 4, 1, 0];
    let rc = ioctl(fd, 0xc020fe00, p.as_mut_ptr().cast());
    if rc < 0 || p[3] != 0 {
        note(b"RF start rc=");
        number(rc);
        note(b"RF start status=");
        number(p[3] as i32);
        return false;
    }
    true
}
#[unsafe(no_mangle)]
#[cfg(not(fetpf_client))]
extern "C" fn probe() -> ! {
    let uart = syscall(5, c"/dev/ttyS0".as_ptr() as i32, 0x902, 0);
    if uart < 0 {
        note(b"UART open=");
        number(uart);
        exit(20);
    }
    let mut old = [0u32; 9];
    if ioctl(uart, 0x5401, old.as_mut_ptr().cast()) < 0 {
        exit(21);
    }
    let mut raw = [0u32; 9];
    raw[2] = 0x18b2;
    if ioctl(uart, 0x5402, raw.as_mut_ptr().cast()) < 0 {
        exit(22);
    }
    let fd = syscall(5, c"/dev/feclr0".as_ptr() as i32, 2, 0);
    if fd < 0 {
        note(b"RF open=");
        number(fd);
        ioctl(uart, 0x5402, old.as_mut_ptr().cast());
        exit(23);
    }
    let began = now();
    let mut sequence = 0;
    let mut scans = 0;
    if start(fd) {
        note(b"FeliCa RF discovery ready\n");
        send(uart, &mut sequence, &[1, 0]);
        let mut last_ready = now();
        while now().wrapping_sub(began) < 900 && scans < 64 {
            // libfeclr wait: timeout, status, technology, 512 discovery bytes, timestamp.
            let mut p = [0u64; 69];
            p[0] = 200_000;
            let rc = ioctl(fd, 0xc228fe01, p.as_mut_ptr().cast());
            if rc < 0 {
                note(b"RF wait rc=");
                number(rc);
                break;
            }
            let mut drain = [0u8; 256];
            syscall(3, uart, drain.as_mut_ptr() as i32, 256);
            if now().wrapping_sub(last_ready) >= 5 {
                send(uart, &mut sequence, &[1, 0]);
                last_ready = now();
            }
            if p[1] != 0 || p[2] != 4 {
                sleep();
                continue;
            }
            let mut payload = [0u8; 514];
            payload[0] = 1;
            payload[1] = 4;
            unsafe {
                core::ptr::copy_nonoverlapping(
                    p.as_ptr().cast::<u8>().add(24),
                    payload.as_mut_ptr().add(2),
                    512,
                );
            }
            send(uart, &mut sequence, &payload);
            scans += 1;
            note(b"FeliCa scan count=");
            number(scans);
            // Do not re-enroll/re-emit a held card. Wait for actual removal.
            loop {
                if now().wrapping_sub(began) >= 900 {
                    break;
                }
                let mut removal = [200_000u64, 0];
                let rc = ioctl(fd, 0xc010fe04, removal.as_mut_ptr().cast());
                if rc < 0 || removal[1] == 0 {
                    break;
                }
            }
            ioctl(fd, 0xfe01, core::ptr::null_mut());
            if !start(fd) {
                break;
            }
        }
    }
    ioctl(fd, 0xfe01, core::ptr::null_mut());
    syscall(6, fd, 0, 0);
    ioctl(uart, 0x5402, old.as_mut_ptr().cast());
    syscall(6, uart, 0, 0);
    note(b"FeliCa discovery trial ended\n");
    exit(0);
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(95);
}
