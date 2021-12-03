use std::borrow::Cow;
use std::ffi::CStr;
use std::path::Path;
use std::ptr::null_mut;


extern crate libssh2_sys as raw;

use crate::{Error, ErrorCode};
#[cfg(unix)]
pub fn path2bytes(p: &Path) -> Result<Cow<[u8]>, Error> {
    use std::ffi::OsStr;
    use std::os::unix::prelude::*;
    let s: &OsStr = p.as_ref();
    check(Cow::Borrowed(s.as_bytes()))
}
#[cfg(windows)]
pub fn path2bytes(p: &Path) -> Result<Cow<[u8]>, Error> {
    p.to_str()
        .map(|s| s.as_bytes())
        .ok_or_else(|| {
            Error::new(
                ErrorCode::Session(raw::LIBSSH2_ERROR_INVAL),
                "only unicode paths on windows may be used",
            )
        })
        .map(|bytes| {
            if bytes.contains(&b'\\') {
                // Normalize to Unix-style path separators
                let mut bytes = bytes.to_owned();
                for b in &mut bytes {
                    if *b == b'\\' {
                        *b = b'/';
                    }
                }
                Cow::Owned(bytes)
            } else {
                Cow::Borrowed(bytes)
            }
        })
        .and_then(check)
}

fn check(b: Cow<[u8]>) -> Result<Cow<[u8]>, Error> {
    if b.iter().any(|b| *b == 0) {
        Err(Error::new(
            ErrorCode::Session(raw::LIBSSH2_ERROR_INVAL),
            "path provided contains a 0 byte",
        ))
    } else {
        Ok(b)
    }
}

pub unsafe fn make_error_message(msg: *mut libc::c_char) -> Cow<'static, str> {
    const FALLBACK: Cow<'_, str> = Cow::Borrowed("<failed to fetch the error message>");
    opt_bytes(&(), msg)
        .and_then(|msg| {
           std::str::from_utf8(msg)
                .map(|msg| Cow::Owned(msg.to_owned()))
                .ok()
        })
        .unwrap_or_else(|| FALLBACK)
}
pub unsafe fn opt_bytes<'a, T>(_: &'a T, c: *const libc::c_char) -> Option<&'a [u8]> {
    if c.is_null() {
        None
    } else {
        Some(CStr::from_ptr(c).to_bytes())
    }
}

pub fn last_session_error_raw(raw: *mut raw::LIBSSH2_SESSION) -> Option<(i32, String)> {
    unsafe {
        let mut msg = null_mut();
        let rc = raw::libssh2_session_last_error(raw, &mut msg, null_mut(), 0);
        if rc == 0 {
            return None;
        }

        // The pointer stored in `msg` points to the internal buffer of
        // LIBSSH2_SESSION, so the error message should be copied before
        // it is overwritten by the next API call.
        Some((rc, make_error_message(msg).parse().unwrap()))
    }
}

pub fn print_error(raw: *mut raw::LIBSSH2_SESSION){
    if let Some((code,msg)) = last_session_error_raw(raw){
        println!("Error code: {} | msg: {}",code,msg);
    }
}