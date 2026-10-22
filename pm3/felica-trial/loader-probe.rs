// Read-only loader diagnosis; no RF, card sessions, authentication or writes.
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() -> ! {
    exit(96)
}
unsafe extern "C" {
    fn dlopen(path: *const core::ffi::c_char, flags: i32) -> *mut core::ffi::c_void;
    fn dlerror() -> *const u8;
    fn dlclose(handle: *mut core::ffi::c_void) -> i32;
    fn __errno_location() -> *mut i32;
    fn getpagesize() -> i32;
}
#[unsafe(no_mangle)]
extern "C" fn service_main(_: i32, _: *const *const u8, _: *const *const u8) -> i32 {
    note(b"Loader page size=");
    number(unsafe { getpagesize() });
    let handle = unsafe { dlopen(c"/home/app0/pm3-adapter.so".as_ptr(), 2) };
    note(b"Loader success=");
    number(i32::from(!handle.is_null()));
    note(b"Loader errno=");
    number(unsafe { *__errno_location() });
    if handle.is_null() {
        let error = unsafe { dlerror() };
        if !error.is_null() {
            let mut n = 0;
            while n < 512 && unsafe { error.add(n).read() } != 0 {
                n += 1;
            }
            note(b"Loader error=");
            note(unsafe { core::slice::from_raw_parts(error, n) });
            note(b"\n");
        }
    } else {
        unsafe {
            dlclose(handle);
        }
    }
    0
}
