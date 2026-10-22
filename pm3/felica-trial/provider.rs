// Explicit cVEND provider binding. No PINs, keys or card attributes are logged.
use core::ffi::c_void;
unsafe extern "C" {
    fn C_GetFunctionList(table: *mut *const u8) -> u32;
}
static mut PROVIDER_TABLE: *const u8 = core::ptr::null();
unsafe fn provider(index: usize) -> Option<usize> {
    if unsafe { PROVIDER_TABLE.is_null() } {
        let mut table = core::ptr::null();
        if unsafe { C_GetFunctionList(&mut table) } != 0 || table.is_null() {
            return None;
        }
        if unsafe { table.read() } != 2 {
            return None;
        }
        unsafe {
            PROVIDER_TABLE = table;
        }
    }
    // PKCS #11 v2 CK_VERSION (2 bytes + padding), then native function pointers.
    let pointer = unsafe { PROVIDER_TABLE.add(4 + index * 4).cast::<usize>().read() };
    if pointer == 0 { None } else { Some(pointer) }
}
macro_rules! forward {
    ($name:ident,$index:literal,($($arg:ident:$ty:ty),*)) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg:$ty),*)->u32 {
            let Some(pointer)=(unsafe {provider($index)}) else {return 6;};
            let function:unsafe extern "C" fn($($ty),*)->u32=unsafe {core::mem::transmute(pointer)};
            unsafe {function($($arg),*)}
        }
    }
}
forward!(C_Initialize,0,(args:*mut c_void));
forward!(C_Finalize,1,(args:*mut c_void));
forward!(C_GetTokenInfo,6,(slot:u32,info:*mut u8));
forward!(C_OpenSession,12,(slot:u32,flags:u32,app:*mut c_void,notify:*mut c_void,session:*mut u32));
forward!(C_CloseSession,13,(session:u32));
forward!(C_Login,18,(session:u32,user:u32,pin:*mut u8,length:u32));
forward!(C_CreateObject,20,(session:u32,attrs:*mut c_void,count:u32,object:*mut u32));
forward!(C_DestroyObject,22,(session:u32,object:u32));
forward!(C_GetAttributeValue,24,(session:u32,object:u32,attrs:*mut c_void,count:u32));
forward!(C_SetAttributeValue,25,(session:u32,object:u32,attrs:*mut c_void,count:u32));
forward!(C_FindObjectsInit,26,(session:u32,attrs:*mut c_void,count:u32));
forward!(C_FindObjects,27,(session:u32,objects:*mut u32,max:u32,count:*mut u32));
forward!(C_FindObjectsFinal,28,(session:u32));
