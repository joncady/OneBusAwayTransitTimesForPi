//! Experimental cVEND app 02.35 adapter, loaded only by a reversible trial launcher.
//! ABI derived from the exact backed-up binary and libraries. Not portable firmware.
#![no_std]
use core::ffi::{c_char, c_void};

unsafe extern "C" {
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn _exit(status: i32) -> !;
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { _exit(90) }
}

type Found = unsafe extern "C" fn(*mut c_void, u64, u64, *const u8, *const c_void, *mut u32) -> i32;
type Process = unsafe extern "C" fn(*mut c_void, *const Callbacks) -> i32;
type Register = unsafe extern "C" fn(*mut c_void, u64) -> i32;
type Send = unsafe extern "C" fn(*mut c_void, u8, u16, *const u8, u8, u8) -> bool;
#[repr(C)]
pub struct Callbacks {
    surrender: usize,
    found: usize,
}
// Only wrap the known stock polling-thread callback. Both fields are used on
// that single thread; avoid ARMv6+ atomic instructions on this ARMv5TE module.
static mut FOUND: usize = 0;
static mut READY: bool = false;
static mut REGISTERED: bool = false;

unsafe fn compatible() -> bool {
    // Secondary guards; deployment additionally requires a full SHA-256 match.
    unsafe {
        (0x2eba0 as *const u32).read() == 0xe3a0202e && (0x23944 as *const u32).read() == 0xe590c404
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fetpf_register_card_processor(context: *mut c_void, mask: u64) -> i32 {
    let symbol = unsafe {
        dlsym(
            (-1isize) as *mut c_void,
            c"fetpf_register_card_processor".as_ptr(),
        )
    };
    if symbol.is_null() {
        unsafe { _exit(91) }
    }
    let original: Register = unsafe { core::mem::transmute(symbol) };
    let matching = mask == 0x2e && unsafe { compatible() };
    let mask = if matching { mask | 0x40 } else { mask };
    let result = unsafe { original(context, mask) };
    if matching {
        unsafe {
            REGISTERED = result == 0;
        }
    }
    result
}

unsafe extern "C" fn found(
    context: *mut c_void,
    category: u64,
    technology: u64,
    data: *const u8,
    time: *const c_void,
    result: *mut u32,
) -> i32 {
    if category == 0x40 && technology == 4 && !data.is_null() && unsafe { compatible() } {
        // Keep the discovery union private until its IDm layout is verified by taps.
        // No authenticated commands, balance/history reads or card writes.
        let mut payload = [0u8; 514];
        payload[0] = 1; // Diagnostic protocol version.
        payload[1] = 4; // FeliCa RF technology.
        unsafe {
            core::ptr::copy_nonoverlapping(data, payload.as_mut_ptr().add(2), 512);
        }
        let send: Send = unsafe { core::mem::transmute(0x23944usize) };
        unsafe {
            send(0x12bdac as *mut c_void, 0xf1, 514, payload.as_ptr(), 0, 0);
        }
    }
    let original = unsafe { FOUND };
    if original == 0 {
        unsafe { _exit(92) }
    }
    let original: Found = unsafe { core::mem::transmute(original) };
    // Preserve stock card ownership/removal. The host releases the subsequent BE
    // event without enrolling its constant/empty fallback identity during trial.
    unsafe { original(context, category, technology, data, time, result) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fetpf_process_card(
    context: *mut c_void,
    callbacks: *const Callbacks,
) -> i32 {
    let symbol = unsafe { dlsym((-1isize) as *mut c_void, c"fetpf_process_card".as_ptr()) };
    if symbol.is_null() {
        unsafe { _exit(93) }
    }
    let original: Process = unsafe { core::mem::transmute(symbol) };
    if callbacks.is_null() || !unsafe { compatible() } {
        return unsafe { original(context, callbacks) };
    }
    let callbacks = unsafe { &*callbacks };
    if callbacks.found != 0x2f038 {
        return unsafe { original(context, callbacks) };
    }
    if !unsafe { READY } && unsafe { REGISTERED } {
        let send: Send = unsafe { core::mem::transmute(0x23944usize) };
        unsafe {
            READY = send(0x12bdac as *mut c_void, 0xf1, 2, [1, 0].as_ptr(), 0, 0);
        }
    }
    unsafe {
        FOUND = callbacks.found;
    }
    let wrapped = Callbacks {
        surrender: callbacks.surrender,
        found: found as usize,
    };
    unsafe { original(context, &wrapped) }
}
