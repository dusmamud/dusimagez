//! Minimal libc shim for the `wasm32-unknown-unknown` build.
//!
//! The C libraries linked into the WASM build (libwebp, via the
//! `webp` crate) call `malloc` / `free` / `calloc` / `realloc` /
//! `qsort` / `bsearch`, which Rust `std` does not provide for
//! `wasm32-unknown-unknown`. These are plain, correct
//! implementations: allocation over `std::alloc` (size stored in a
//! 16-byte header), sorting/searching over the C comparator.
//!
//! Build with `scripts/wasm-build.sh`, which compiles this crate
//! for `wasm32-unknown-unknown` and passes the resulting `.rlib` to
//! the linker (`-C link-arg=...`). Never used on any other target.

#![allow(clippy::missing_safety_doc)]
use std::ffi::{c_int, c_void};

const ALIGN: usize = 16;

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    let want = if size == 0 { 1 } else { size };
    let Some(total) = want.checked_add(ALIGN) else {
        return std::ptr::null_mut();
    };
    let Ok(layout) = std::alloc::Layout::from_size_align(total, ALIGN) else {
        return std::ptr::null_mut();
    };
    let ptr = std::alloc::alloc(layout);
    if ptr.is_null() {
        return ptr as *mut c_void;
    }
    *(ptr as *mut usize) = total;
    ptr.add(ALIGN) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let base = (ptr as *mut u8).sub(ALIGN);
    let total = *(base as *const usize);
    if let Ok(layout) = std::alloc::Layout::from_size_align(total, ALIGN) {
        std::alloc::dealloc(base, layout);
    }
}

#[no_mangle]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
    let Some(total) = nmemb.checked_mul(size) else {
        return std::ptr::null_mut();
    };
    let ptr = malloc(total);
    if !ptr.is_null() {
        std::ptr::write_bytes(ptr as *mut u8, 0, total);
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return malloc(size);
    }
    if size == 0 {
        free(ptr);
        return std::ptr::null_mut();
    }
    let base = (ptr as *mut u8).sub(ALIGN);
    let old_total = *(base as *const usize);
    let old_size = old_total.saturating_sub(ALIGN);
    let new_ptr = malloc(size);
    if new_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let copy = old_size.min(size);
    std::ptr::copy_nonoverlapping(ptr as *const u8, new_ptr as *mut u8, copy);
    free(ptr);
    new_ptr
}

type Comparator = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

#[no_mangle]
pub unsafe extern "C" fn qsort(
    base: *mut c_void,
    nmemb: usize,
    size: usize,
    compar: Option<Comparator>,
) {
    if nmemb <= 1 || size == 0 {
        return;
    }
    let Some(cmp) = compar else { return };
    let mut idx: Vec<usize> = (0..nmemb).collect();
    idx.sort_by(|&a, &b| {
        let pa = (base as *const u8).add(a * size) as *const c_void;
        let pb = (base as *const u8).add(b * size) as *const c_void;
        cmp(pa, pb).cmp(&0)
    });
    let mut tmp = vec![0u8; nmemb * size];
    for (i, &src) in idx.iter().enumerate() {
        let s = (base as *const u8).add(src * size);
        std::ptr::copy_nonoverlapping(s, tmp.as_mut_ptr().add(i * size), size);
    }
    std::ptr::copy_nonoverlapping(tmp.as_ptr(), base as *mut u8, nmemb * size);
}

#[no_mangle]
pub unsafe extern "C" fn bsearch(
    key: *const c_void,
    base: *const c_void,
    nmemb: usize,
    size: usize,
    compar: Option<Comparator>,
) -> *mut c_void {
    let Some(cmp) = compar else {
        return std::ptr::null_mut();
    };
    let mut lo = 0usize;
    let mut hi = nmemb;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let elem = (base as *const u8).add(mid * size) as *const c_void;
        let ord = cmp(key, elem);
        if ord == 0 {
            return elem as *mut c_void;
        } else if ord < 0 {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    std::ptr::null_mut()
}
