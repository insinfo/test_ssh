#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]


use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, RawSocket};

use std::path::{Path, PathBuf};
use std::{io, slice};
use std::io::Write;
use std::str;
use std::net::TcpStream;
use std::ptr::null_mut;

extern crate libssh2_sys as raw;

use libc::{self, c_char, c_int, c_long, c_uint, c_void, size_t};
use libssh2_sys::*;
use test_ssh::{FsEntry, FsEntryType};

use test_ssh::utils::{make_error_message, path2bytes, print_error};


pub fn run() {
    let mut ssh = Libssh2::new().unwrap();
    let tcp = TcpStream::connect("192.168.133.13:22").unwrap();
    ssh.connect(tcp, "isaque.neves", "Ins257257");

    let dir_to_download = Path::new("/etc/apache2");
    let dest_dir_path = Path::new(r"C:\MyRustProjects\test_ssh\download");
    std::fs::create_dir_all(&dest_dir_path).unwrap();

    let item = FsEntry{
        path: PathBuf::from("/etc/apache2/apache2.conf"),
        file_type: FsEntryType::File,
        is_link: false
    };

    let mut dst_path = PathBuf::from(&item.path.strip_prefix(&dir_to_download).unwrap());
    dst_path = dest_dir_path.join(dst_path);
    println!("dst_path {:?}",dst_path);
    ssh.download_item(&item,&dst_path);

    ssh.disconnect();

    //download_item(&Path::new("/etc/apache2/apache2.conf"));
    //raw::libssh2_channel_free(channel);
    //channel = std::ptr::null_mut();
}

pub struct Libssh2 {
    pub session: *mut raw::LIBSSH2_SESSION,
    #[cfg(unix)]
    tcp: Option<Box<dyn AsRawFd>>,
    #[cfg(windows)]
    tcp: Option<Box<dyn AsRawSocket>>,
    channel: *mut raw::LIBSSH2_CHANNEL,
}

impl Libssh2 {
    pub fn new() -> Result<Libssh2, ()> {
        unsafe {
            let session = raw::libssh2_session_init_ex(None, None, None, 0 as *mut _);
            if session.is_null() {
                println!("Error on init libssh2_session");
            }
            Ok(Libssh2 {
                session,
                tcp: None,
                channel: std::ptr::null_mut(),
            })
        }
    }

    pub fn connect<S: 'static + AsRawSocket>(&mut self, stream: S, user: &str, pass: &str) {
        unsafe {
            self.tcp = Some(Box::new(stream));
            let mut rc = raw::libssh2_session_handshake(self.session, self.tcp.as_ref().unwrap().as_raw_socket());
            if rc != 0 {
                println!("Failure establishing SSH session: {}", rc);
                print_error(self.session);
            }

            rc = raw::libssh2_userauth_password_ex(self.session, user.as_ptr() as *const _,
                                                   user.len() as c_uint,
                                                   pass.as_ptr() as *const _,
                                                   pass.len() as c_uint,
                                                   None, );
            if rc != 0 {
                println!("Authentication by password failed: {}", rc);
                print_error(self.session);
            }
        }
    }

    fn download_item(&mut self, entry: &FsEntry, dst_path: &Path) {
        unsafe {
            let path = CString::new(path2bytes(&entry.path).unwrap()).unwrap();
            //#[allow(deprecated)]
            //std::mem::uninitialized()
            let mut fileinfo: libssh2_struct_stat = std::mem::zeroed();

            let channel = raw::libssh2_scp_recv2(self.session, path.as_ptr(), &mut fileinfo);
            if channel.is_null() {
                println!("Failed to recv file: ");
                print_error(self.session);
                return;
            }
            let mut dest_file = std::fs::File::create(&dst_path).unwrap();

            let mut got = 0;
            while got < fileinfo.st_size {
                #[allow(deprecated)]
                    let mut buffer: [u8; 1024] = std::mem::uninitialized();
                let mut amount = std::mem::size_of_val(&buffer) as i64;

                if (fileinfo.st_size - got) < amount {
                    amount = (fileinfo.st_size - got) as i64;
                }

                let rc = raw::libssh2_channel_read_ex(channel, 0, buffer.as_mut_ptr() as *mut _, amount as size_t) as i64;

                if rc > 0 {
                    //let mut out_writer = Box::new(io::stdout()) as Box<dyn Write>;
                    dest_file.write(&buffer[..rc as usize]).unwrap();
                } else if rc < 0 {
                    println!("libssh2_channel_read() failed: {}", rc);
                    print_error(self.session);
                    break;
                }
                got += rc;
            }
            raw::libssh2_channel_free(channel);
        }
    }

    fn disconnect(&mut self) {
        unsafe {
            let msg = CString::new("Normal Shutdown").unwrap();
            let lang = CString::new("").unwrap();
            raw::libssh2_session_disconnect_ex(self.session, raw::SSH_DISCONNECT_BY_APPLICATION, msg.as_ptr(), lang.as_ptr());
            raw::libssh2_session_free(self.session);
        }
    }
}




