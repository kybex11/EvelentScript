use std::os::raw::c_char;

use evelent_native::{parse_argv, return_json, NativeExport, NativeModuleInfo, ABI_VERSION};

unsafe extern "C" fn greet(argv: *const c_char) -> *mut c_char {
    let args = parse_argv(argv).unwrap_or_default();
    let name = args
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("world");
    return_json(serde_json::json!(format!("hello, {name}")))
}

unsafe extern "C" fn add(argv: *const c_char) -> *mut c_char {
    let args = parse_argv(argv).unwrap_or_default();
    let a = args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let b = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
    return_json(serde_json::json!(a + b))
}

#[no_mangle]
pub unsafe extern "C" fn evelent_native_init() -> *const NativeModuleInfo {
    static EXPORTS: [NativeExport; 2] = [
        NativeExport {
            name: b"greet\0".as_ptr() as *const c_char,
            func: greet,
        },
        NativeExport {
            name: b"add\0".as_ptr() as *const c_char,
            func: add,
        },
    ];
    static INFO: NativeModuleInfo = NativeModuleInfo {
        abi_version: ABI_VERSION,
        name: b"hello_native\0".as_ptr() as *const c_char,
        exports: EXPORTS.as_ptr(),
        export_count: 2,
    };
    &INFO
}
