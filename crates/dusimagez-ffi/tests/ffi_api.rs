//! Integration tests for the dusimagez C ABI, called through the
//! `rlib` build of the FFI crate (same symbols as the cdylib).
//!
//! Fixture: a 1x1 greyscale+alpha PNG (68 bytes, colour type "La8"),
//! embedded as a byte array so no binary blob is committed. Run with
//! `cargo test -p dusimagez-ffi` once the core crate exists.

use std::ffi::CStr;
use std::ptr;

use dusimagez_ffi::{
    DusimagezInfo, DusimagezOptions, DusimagezReport, STATUS_EMPTY_INPUT, STATUS_INVALID_IMAGE,
    STATUS_INVALID_QUALITY, STATUS_OK, STATUS_TARGET_SIZE_UNREACHABLE, dusimagez_compress,
    dusimagez_compress_to_size, dusimagez_compress_with_report, dusimagez_free, dusimagez_get_info,
    dusimagez_last_error, dusimagez_version,
};

// 1x1 greyscale+alpha PNG (colour type 4 = "La8"), 68 bytes.
const PNG_1X1: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4, 0,
    0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 96, 96, 0, 0, 0, 3, 0, 1, 184,
    173, 58, 99, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

const GARBAGE: &[u8] = b"this is not an image at all, just plain text bytes 1234567890";

fn last_error_code() -> String {
    let ptr = dusimagez_last_error();
    assert!(!ptr.is_null(), "last_error must be set after a failure");
    let json: serde_json::Value =
        serde_json::from_str(unsafe { CStr::from_ptr(ptr) }.to_str().unwrap()).unwrap();
    json["code"].as_str().unwrap().to_string()
}

fn default_opts() -> DusimagezOptions {
    DusimagezOptions::default()
}

#[test]
fn compress_defaults_round_trips() {
    let mut out: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let opts = default_opts();
    let status = unsafe {
        dusimagez_compress(
            PNG_1X1.as_ptr(),
            PNG_1X1.len(),
            &opts,
            &mut out,
            &mut out_len,
        )
    };
    assert_eq!(status, STATUS_OK, "code={}", last_error_code());
    assert!(!out.is_null() && out_len > 0);
    // Output must still decode as a 1x1 PNG (same format kept).
    assert_eq!(
        &unsafe { std::slice::from_raw_parts(out, 8) },
        b"\x89PNG\r\n\x1a\n"
    );
    unsafe { dusimagez_free(out, out_len) };
    assert!(dusimagez_last_error().is_null());
}

#[test]
fn compress_null_options_means_defaults() {
    let mut out: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let status = unsafe {
        dusimagez_compress(
            PNG_1X1.as_ptr(),
            PNG_1X1.len(),
            ptr::null(),
            &mut out,
            &mut out_len,
        )
    };
    assert_eq!(status, STATUS_OK);
    assert!(out_len > 0);
    unsafe { dusimagez_free(out, out_len) };
}

#[test]
fn compress_empty_input_is_empty_input() {
    let mut out: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let status = unsafe { dusimagez_compress(ptr::null(), 0, ptr::null(), &mut out, &mut out_len) };
    assert_eq!(status, STATUS_EMPTY_INPUT);
    assert_eq!(last_error_code(), "EMPTY_INPUT");
    assert!(out.is_null());
}

#[test]
fn compress_garbage_is_invalid_image() {
    let mut out: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let status = unsafe {
        dusimagez_compress(
            GARBAGE.as_ptr(),
            GARBAGE.len(),
            ptr::null(),
            &mut out,
            &mut out_len,
        )
    };
    assert_eq!(status, STATUS_INVALID_IMAGE);
    assert_eq!(last_error_code(), "INVALID_IMAGE");
}

#[test]
fn compress_invalid_quality_is_rejected_in_ffi() {
    let mut opts = default_opts();
    opts.quality = 0;
    let mut out: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let status = unsafe {
        dusimagez_compress(
            PNG_1X1.as_ptr(),
            PNG_1X1.len(),
            &opts,
            &mut out,
            &mut out_len,
        )
    };
    assert_eq!(status, STATUS_INVALID_QUALITY);
    assert_eq!(last_error_code(), "INVALID_QUALITY");

    opts.quality = 101;
    let status = unsafe {
        dusimagez_compress(
            PNG_1X1.as_ptr(),
            PNG_1X1.len(),
            &opts,
            &mut out,
            &mut out_len,
        )
    };
    assert_eq!(status, STATUS_INVALID_QUALITY);
}

#[test]
fn compress_to_size_unreachable_target() {
    // A 1x1 image cannot shrink below 10 bytes, and cannot downscale
    // below 10% of 1px — so a 10-byte target must be unreachable.
    let mut out: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let status = unsafe {
        dusimagez_compress_to_size(
            PNG_1X1.as_ptr(),
            PNG_1X1.len(),
            10,
            ptr::null(),
            &mut out,
            &mut out_len,
        )
    };
    assert_eq!(status, STATUS_TARGET_SIZE_UNREACHABLE);
    assert_eq!(last_error_code(), "TARGET_SIZE_UNREACHABLE");
}

#[test]
fn compress_with_report_fields_agree() {
    let mut report = DusimagezReport {
        output: ptr::null_mut(),
        output_len: 0,
        original_size: 0,
        output_size: 0,
        saved_bytes: 0,
        saved_percent: 0.0,
        format: 0,
        width: 0,
        height: 0,
        quality_used: 0,
        resized: 0,
    };
    let opts = default_opts();
    let status = unsafe {
        dusimagez_compress_with_report(PNG_1X1.as_ptr(), PNG_1X1.len(), &opts, &mut report)
    };
    assert_eq!(status, STATUS_OK, "code={}", last_error_code());
    assert_eq!(report.original_size, PNG_1X1.len() as u64);
    assert_eq!(report.output_size, report.output_len as u64);
    assert_eq!(report.format, 2, "kept input format: png = 2");
    assert_eq!((report.width, report.height), (1, 1));
    assert_eq!(report.quality_used, 80);
    assert!(!report.output.is_null());
    unsafe { dusimagez_free(report.output, report.output_len) };
}

#[test]
fn get_info_png_1x1() {
    let mut info = DusimagezInfo {
        format: 0,
        width: 0,
        height: 0,
        has_alpha: 0,
        colour_type: [0; 24],
        file_size: 0,
    };
    let status = unsafe { dusimagez_get_info(PNG_1X1.as_ptr(), PNG_1X1.len(), &mut info) };
    assert_eq!(status, STATUS_OK, "code={}", last_error_code());
    assert_eq!(info.format, 2, "png = 2");
    assert_eq!((info.width, info.height), (1, 1));
    assert_eq!(info.has_alpha, 1, "La8 has alpha");
    let colour = unsafe { CStr::from_ptr(info.colour_type.as_ptr()) }
        .to_str()
        .unwrap();
    assert_eq!(colour, "la8"); // FFI returns the lowercase colour type
    assert_eq!(info.file_size, PNG_1X1.len() as u64);
}

#[test]
fn bulk_compress_mixed_ok_and_error() {
    // One good input, one garbage input: the good item succeeds, the
    // bad item fails per-item, and the call itself returns OK.
    let inputs: [*const u8; 2] = [PNG_1X1.as_ptr(), GARBAGE.as_ptr()];
    let lens: [usize; 2] = [PNG_1X1.len(), GARBAGE.len()];
    let mut out: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;
    let status = unsafe {
        dusimagez_ffi::dusimagez_bulk_compress(
            inputs.as_ptr(),
            lens.as_ptr(),
            2,
            ptr::null(),
            &mut out,
            &mut out_len,
        )
    };
    assert_eq!(status, STATUS_OK, "code={}", last_error_code());
    let json: serde_json::Value =
        serde_json::from_slice(unsafe { std::slice::from_raw_parts(out, out_len) }).unwrap();
    unsafe { dusimagez_free(out, out_len) };
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["ok"], true);
    assert!(!items[0]["output"].as_str().unwrap().is_empty());
    assert_eq!(items[1]["ok"], false);
    assert_eq!(items[1]["error"]["code"], "INVALID_IMAGE");
}

#[test]
fn free_null_is_noop_and_version_is_0_1_0() {
    unsafe { dusimagez_free(ptr::null_mut(), 0) };
    let v = unsafe { CStr::from_ptr(dusimagez_version()) }
        .to_str()
        .unwrap();
    assert_eq!(v, "0.1.0");
}
