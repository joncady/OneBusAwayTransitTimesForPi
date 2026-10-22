//! ARMv5-only Linux syscall probe: maps a trial-owned file, never opens NFC hardware.
#![no_std]
#![no_main]
use core::arch::global_asm;
global_asm!(
    r#"
    .syntax unified
    .arm
    .global _start
_start:
    bl probe
    b .
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
unsafe extern "C" {
    fn call6(n: i32, a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32;
}
fn text(s: &[u8]) {
    unsafe {
        call6(4, 1, s.as_ptr() as i32, s.len() as i32, 0, 0, 0);
    }
}
fn number(mut n: i32) {
    if n < 0 {
        text(b"-");
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
    text(&b[i..]);
    text(b"\n");
}
#[unsafe(no_mangle)]
extern "C" fn probe() -> ! {
    let fd = unsafe {
        call6(
            5,
            c"/tmp/pm3-adapter-trial.so".as_ptr() as i32,
            0,
            0,
            0,
            0,
            0,
        )
    };
    text(b"probe-open=");
    number(fd);
    if fd >= 0 {
        for prot in [1, 5] {
            let address = unsafe { call6(192, 0, 32768, prot, 2, fd, 0) };
            text(if prot == 1 {
                b"probe-map-r="
            } else {
                b"probe-map-rx="
            });
            if (-4095..0).contains(&address) {
                number(address);
            } else {
                text(b"ok\n");
                unsafe {
                    call6(91, address, 32768, 0, 0, 0, 0);
                }
            }
        }
        unsafe {
            call6(6, fd, 0, 0, 0, 0, 0);
        }
    }
    unsafe {
        call6(1, 0, 0, 0, 0, 0, 0);
    }
    loop {
        core::hint::spin_loop();
    }
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe {
        call6(1, 94, 0, 0, 0, 0, 0);
    }
    loop {
        core::hint::spin_loop();
    }
}
