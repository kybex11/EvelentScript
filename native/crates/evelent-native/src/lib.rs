//! ABI for native EvelentScript host modules (`cdylib` plugins).
//!
//! Load from EvelentScript with `require 'native:module_name'`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Semantic version of the native module ABI.
pub const ABI_VERSION: u32 = 1;

/// Native function: `argv` is a JSON array string; returns a heap-allocated JSON string
/// freed by the host via [`evelent_native_string_free`].
pub type NativeFn = unsafe extern "C" fn(argv_json: *const c_char) -> *mut c_char;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeExport {
    pub name: *const c_char,
    pub func: NativeFn,
}

// SAFETY: export tables are immutable static data shared read-only.
unsafe impl Send for NativeExport {}
unsafe impl Sync for NativeExport {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeModuleInfo {
    pub abi_version: u32,
    pub name: *const c_char,
    pub exports: *const NativeExport,
    pub export_count: usize,
}

unsafe impl Send for NativeModuleInfo {}
unsafe impl Sync for NativeModuleInfo {}

/// Entry point every native module must export.
pub type NativeInitFn = unsafe extern "C" fn() -> *const NativeModuleInfo;

/// Free a string returned by a native export.
#[no_mangle]
pub unsafe extern "C" fn evelent_native_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Parse argv JSON into a list of values.
pub fn parse_argv(argv_json: *const c_char) -> Result<Vec<serde_json::Value>, String> {
    if argv_json.is_null() {
        return Ok(Vec::new());
    }
    let s = unsafe { CStr::from_ptr(argv_json) }
        .to_str()
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value =
        serde_json::from_str(s).map_err(|e| format!("invalid argv json: {e}"))?;
    match v {
        serde_json::Value::Array(a) => Ok(a),
        other => Ok(vec![other]),
    }
}

/// Return a JSON value as an owned C string for the host.
pub fn return_json(value: serde_json::Value) -> *mut c_char {
    let s = value.to_string();
    CString::new(s)
        .unwrap_or_else(|_| CString::new("null").unwrap())
        .into_raw()
}

/// Declare a native module. Expands to `evelent_native_init`.
///
/// ```ignore
/// declare_native_module!("hello_native", {
///     "greet" => greet_fn,
/// });
/// ```
#[macro_export]
macro_rules! declare_native_module {
    ($mod_name:literal, { $($export_name:literal => $export_fn:expr),* $(,)? }) => {
        #[no_mangle]
        pub unsafe extern "C" fn evelent_native_init() -> *const $crate::NativeModuleInfo {
            static EXPORTS: &[$crate::NativeExport] = &[
                $(
                    $crate::NativeExport {
                        name: concat!($export_name, "\0").as_ptr() as *const std::os::raw::c_char,
                        func: $export_fn,
                    },
                )*
            ];
            static INFO: $crate::NativeModuleInfo = $crate::NativeModuleInfo {
                abi_version: $crate::ABI_VERSION,
                name: concat!($mod_name, "\0").as_ptr() as *const std::os::raw::c_char,
                exports: EXPORTS.as_ptr(),
                export_count: {
                    let n = 0usize $( + { let _ = $export_name; 1 } )*;
                    n
                },
            };
            &INFO
        }
    };
}
