pub mod utils;


use std::path::PathBuf;
use libc;
use std::borrow::Cow;
use std::fmt;
use std::str;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum FsEntryType {
    File,
    Directory,
}

#[allow(dead_code)]
pub struct FsEntry {
    pub path: PathBuf,
    pub file_type: FsEntryType,
    pub is_link: bool,
}

// Implement `Display` for `FsEntry`.
impl fmt::Display for FsEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {:?}, {})", self.path.display(), self.file_type, self.is_link)
    }
}



/// An error code originating from a particular source.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    /// Codes for errors that originate in libssh2.
    /// Can be one of  `LIBSSH2_ERROR_*` constants.
    Session(libc::c_int),

    /// Codes for errors that originate in the SFTP subsystem.
    /// Can be one of `LIBSSH2_FX_*` constants.
    //
    // TODO: This should be `c_ulong` instead of `c_int` because these constants
    // are only returned by `libssh2_sftp_last_error()` which returns `c_ulong`.
    SFTP(libc::c_int),
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Representation of an error that can occur within libssh2
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Error {
    code: ErrorCode,
    msg: Cow<'static, str>,
}

impl Error {
    /// Create a new error for the given code and message
    pub fn new(code: ErrorCode, msg: &'static str) -> Error {
        Error {
            code,
            msg: Cow::Borrowed(msg),
        }
    }
}