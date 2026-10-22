// Module-owned FETPF service client; included by rf-probe.rs for the service build.
#[cfg(not(service_proxy))]
mod provider {
    include!("provider.rs");
}
use core::ffi::c_void;
#[cfg(not(service_proxy))]
use provider::{C_GetTokenInfo, C_Initialize};
#[cfg(service_proxy)]
unsafe extern "C" {
    fn C_Initialize(args: *mut c_void) -> u32;
    fn C_GetTokenInfo(slot: u32, info: *mut u8) -> u32;
    fn C_GetSlotList(present: u8, slots: *mut u32, count: *mut u32) -> u32;
}
unsafe extern "C" {
    fn fetpf_new(slot: u32) -> *mut c_void;
    fn fetpf_free(context: *mut c_void);
    fn fetpf_register_card_processor(context: *mut c_void, mask: u64) -> i32;
    fn fetpf_process_card(context: *mut c_void, callbacks: *const ServiceCallbacks) -> i32;
}
#[repr(C)]
struct ServiceCallbacks {
    surrender: unsafe extern "C" fn(*mut c_void, u32) -> i32,
    found: unsafe extern "C" fn(*mut c_void, u64, u64, *const u8, *const c_void, *mut u32) -> i32,
}
static mut SERVICE_UART: i32 = -1;
static mut SERVICE_SEQ: u8 = 0;
static mut SERVICE_SCANS: u32 = 0;
unsafe extern "C" fn surrender(_: *mut c_void, _: u32) -> i32 {
    1
}
unsafe extern "C" fn service_found(
    _: *mut c_void,
    category: u64,
    technology: u64,
    data: *const u8,
    _: *const c_void,
    result: *mut u32,
) -> i32 {
    if category == 0x40 && technology == 4 && !data.is_null() {
        let mut payload = [0u8; 514];
        payload[0] = 1;
        payload[1] = 4;
        unsafe {
            core::ptr::copy_nonoverlapping(data, payload.as_mut_ptr().add(2), 512);
        }
        let mut sequence = unsafe { SERVICE_SEQ };
        send(unsafe { SERVICE_UART }, &mut sequence, &payload);
        unsafe {
            SERVICE_SEQ = sequence;
            SERVICE_SCANS += 1;
        }
        note(b"FeliCa service scan count=");
        number(unsafe { SERVICE_SCANS } as i32);
    }
    // The normal callback's completion path uses result 3 and return 0.
    if !result.is_null() {
        unsafe {
            result.write(3);
        }
    }
    0
}
#[unsafe(no_mangle)]
extern "C" fn service_main(_: i32, _: *const *const u8, _: *const *const u8) -> i32 {
    let uart = syscall(5, c"/dev/ttyS0".as_ptr() as i32, 0x902, 0);
    if uart < 0 {
        note(b"Service UART open=");
        number(uart);
        return 30;
    }
    let mut old = [0u32; 9];
    if ioctl(uart, 0x5401, old.as_mut_ptr().cast()) < 0 {
        return 31;
    }
    let mut raw = [0u32; 9];
    raw[2] = 0x18b2;
    if ioctl(uart, 0x5402, raw.as_mut_ptr().cast()) < 0 {
        return 32;
    }
    // The exact stock getFepkcs11TokenSlotId maps cwd /home/app0 to slot 1.
    // fetpf_new opens a local service session; no card login or fare keys are used.
    unsafe {
        C_Initialize(core::ptr::null_mut());
    }
    let mut info = [0u32; 40];
    let info_rc = unsafe { C_GetTokenInfo(1, info.as_mut_ptr().cast()) };
    note(b"Service token-info rc=");
    number(info_rc as i32);
    if info_rc == 0 {
        // Log counts only. The buffer's label/manufacturer/serial remain private.
        note(b"Service max-sessions=");
        number(info[25] as i32);
        note(b"Service sessions=");
        number(info[26] as i32);
        note(b"Service max-rw=");
        number(info[27] as i32);
        note(b"Service rw=");
        number(info[28] as i32);
    }
    let mut selected_slot = 1u32;
    #[cfg(service_proxy)]
    if info_rc != 0 {
        let mut slots = [0u32; 16];
        let mut count = 16u32;
        let rc = unsafe { C_GetSlotList(1, slots.as_mut_ptr(), &mut count) };
        note(b"Visible service slots rc=");
        number(rc as i32);
        note(b"Visible service slots count=");
        number(count as i32);
        let mut public_firmware = false;
        if rc == 0 && count <= 16 {
            for slot in &slots[..count as usize] {
                let mut public_info = [0u32; 40];
                let rc = unsafe { C_GetTokenInfo(*slot, public_info.as_mut_ptr().cast()) };
                note(b"Visible service slot=");
                number(*slot as i32);
                note(b"Visible service info rc=");
                number(rc as i32);
                if rc == 0 {
                    note(b"Visible service flags=");
                    number(public_info[24] as i32);
                    // Only the public base software token, without a PIN requirement.
                    if *slot == 0 && public_info[24] & 4 == 0 {
                        public_firmware = true;
                    }
                }
            }
        }
        if public_firmware {
            selected_slot = 0;
        } else {
            note(b"No public framework token available\n");
            ioctl(uart, 0x5402, old.as_mut_ptr().cast());
            return 34;
        }
    }
    note(b"Selected framework slot=");
    number(selected_slot as i32);
    let mut context = core::ptr::null_mut();
    for _ in 0..10 {
        context = unsafe { fetpf_new(selected_slot) };
        if !context.is_null() {
            break;
        }
        let delay = [1i32, 0];
        syscall(162, delay.as_ptr() as i32, 0, 0);
    }
    if context.is_null() {
        note(b"FETPF client initialization failed\n");
        ioctl(uart, 0x5402, old.as_mut_ptr().cast());
        return 33;
    }
    let rc = unsafe { fetpf_register_card_processor(context, 0x40) };
    if rc != 0 {
        note(b"FETPF registration=");
        number(rc);
    } else {
        unsafe {
            SERVICE_UART = uart;
        }
        let mut sequence = 0;
        send(uart, &mut sequence, &[1, 0]);
        unsafe {
            SERVICE_SEQ = sequence;
        }
        note(b"FeliCa FETPF service ready\n");
        let callbacks = ServiceCallbacks {
            surrender,
            found: service_found,
        };
        let began = now();
        let mut last_ready = now();
        while now().wrapping_sub(began) < 900 && unsafe { SERVICE_SCANS } < 64 {
            if now().wrapping_sub(last_ready) >= 5 {
                let mut sequence = unsafe { SERVICE_SEQ };
                send(uart, &mut sequence, &[1, 0]);
                unsafe {
                    SERVICE_SEQ = sequence;
                }
                last_ready = now();
            }
            let rc = unsafe { fetpf_process_card(context, &callbacks) };
            if !matches!(rc, 0 | 1 | 3) {
                note(b"FETPF processing=");
                number(rc);
                break;
            }
            let mut drain = [0u8; 256];
            syscall(3, uart, drain.as_mut_ptr() as i32, 256);
            sleep();
        }
    }
    unsafe {
        fetpf_free(context);
    }
    ioctl(uart, 0x5402, old.as_mut_ptr().cast());
    syscall(6, uart, 0, 0);
    note(b"FeliCa FETPF trial ended\n");
    0
}
