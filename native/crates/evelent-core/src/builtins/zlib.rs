use flate2::read::{DeflateDecoder, GzDecoder};
use flate2::write::{DeflateEncoder, GzEncoder};
use flate2::Compression;
use std::io::{Read, Write};

use crate::builtins::fs::{bytes_to_buffer, value_to_bytes};
use crate::error::{Error, Result};
use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "gzipSync",
        native("zlib.gzipSync", 1, |_, args| {
            let input = value_to_bytes(args.first().unwrap_or(&Value::String(String::new())))?;
            let mut enc = GzEncoder::new(Vec::new(), Compression::default());
            enc.write_all(&input).map_err(|e| Error::Other(e.to_string()))?;
            let out = enc.finish().map_err(|e| Error::Other(e.to_string()))?;
            Ok(bytes_to_buffer(&out))
        }),
    );

    let _ = m.set_prop(
        "gunzipSync",
        native("zlib.gunzipSync", 1, |_, args| {
            let input = value_to_bytes(args.first().unwrap_or(&Value::String(String::new())))?;
            let mut dec = GzDecoder::new(&input[..]);
            let mut out = Vec::new();
            dec.read_to_end(&mut out)
                .map_err(|e| Error::Other(e.to_string()))?;
            Ok(bytes_to_buffer(&out))
        }),
    );

    let _ = m.set_prop(
        "deflateSync",
        native("zlib.deflateSync", 1, |_, args| {
            let input = value_to_bytes(args.first().unwrap_or(&Value::String(String::new())))?;
            let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
            enc.write_all(&input).map_err(|e| Error::Other(e.to_string()))?;
            let out = enc.finish().map_err(|e| Error::Other(e.to_string()))?;
            Ok(bytes_to_buffer(&out))
        }),
    );

    let _ = m.set_prop(
        "inflateSync",
        native("zlib.inflateSync", 1, |_, args| {
            let input = value_to_bytes(args.first().unwrap_or(&Value::String(String::new())))?;
            let mut dec = DeflateDecoder::new(&input[..]);
            let mut out = Vec::new();
            dec.read_to_end(&mut out)
                .map_err(|e| Error::Other(e.to_string()))?;
            Ok(bytes_to_buffer(&out))
        }),
    );

    m
}

#[allow(dead_code)]
fn _r(_: Result<Value>) {}
