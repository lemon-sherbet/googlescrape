#![cfg(feature = "ffi")]

use libc::c_char;
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;

#[no_mangle]
#[repr(C)]
#[derive(Debug)]
pub struct GResult {
    pub title: *mut c_char,
    pub link: *mut c_char,
    pub description: *mut c_char,
}

impl Drop for GResult {
    fn drop(&mut self) {
        unsafe {
            mem::drop(CString::from_raw(self.title));
            mem::drop(CString::from_raw(self.link));
            mem::drop(CString::from_raw(self.description));
        }
    }
}

#[no_mangle]
pub extern "C" fn freeGResults(ret: returned) {
    mem::drop(ret)
}

#[repr(C)]
#[no_mangle]
pub struct returned {
    ret: *mut GResult,
    err: *mut c_char,
}

impl Drop for returned {
    fn drop(&mut self) {
        if !self.err.is_null() {
            mem::drop(unsafe { Vec::from_raw_parts(self.ret, 3, 3) });
            mem::drop(unsafe { CString::from_raw(self.err) });
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn google(query: *const c_char) -> returned {
    match unsafe {
        CStr::from_ptr(query)
            .to_str()
            .map_err(Box::from)
            .and_then(super::_google)
    } {
        Ok(mut v) => returned {
            ret: v.as_mut_ptr(),
            err: ptr::null_mut(),
        },
        Err(x) => returned {
            ret: ptr::null_mut(),
            err: CString::new(x.to_string()).unwrap().into_raw(),
        },
    }
}
