#![allow(dead_code)]
#![allow(unused_imports)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, deprecated))]

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

extern crate libssh2_sys;

use libc::{self, c_char, c_int, c_long, c_uint, c_void, size_t};
use libssh2_sys::*;
use test_ssh::{FsEntry, FsEntryType};

use test_ssh::utils::{make_error_message, path2bytes, print_error};

const BUFFER_SIZE: usize = (1024 * 128) * 2;// 100 * 1024;  128 *

pub fn run() {
    let mut ssh = Libssh2::new().unwrap();
    let tcp = TcpStream::connect("192.168.133.13:22").unwrap();
    ssh.connect(tcp, "isaque.neves", "Ins257257");

    let dir_to_download = Path::new("/var/www/html/portalPmro");
    let dest_dir_path = Path::new(r"C:\MyRustProjects\test_ssh\download");
    std::fs::create_dir_all(&dest_dir_path).unwrap();


    let start = std::time::Instant::now();
    let items_to_download = ssh.list_dir(&dir_to_download);
    println!("Time elapsed in list dir: {:?}", start.elapsed());

    let start = std::time::Instant::now();
    for item in items_to_download.iter() {
        //remove a parte inicial do caminho
        let mut dst_path = PathBuf::from(&item.path.strip_prefix(&dir_to_download).unwrap());
        dst_path = dest_dir_path.join(dst_path);
        ssh.download_item(&item, &dst_path);
        println!("item {}", item.path.display());
    }
    println!("Time elapsed in file transfer: {:?}", start.elapsed());
    println!("download of {:?} complete!", dir_to_download);

    ssh.disconnect();
}

pub struct Libssh2 {
    pub session: *mut LIBSSH2_SESSION,
    #[cfg(unix)]
    tcp: Option<Box<dyn AsRawFd>>,
    #[cfg(windows)]
    tcp: Option<Box<dyn AsRawSocket>>,
    channel: *mut LIBSSH2_CHANNEL,
}

impl Libssh2 {
    pub fn new() -> Result<Libssh2, ()> {
        unsafe {
            let session = libssh2_session_init_ex(None, None, None, 0 as *mut _);
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
            let mut rc = libssh2_session_handshake(self.session, self.tcp.as_ref().unwrap().as_raw_socket());
            if rc != 0 {
                println!("Failure establishing SSH session: {}", rc);
                print_error(self.session);
            }

            rc = libssh2_userauth_password_ex(self.session, user.as_ptr() as *const _,
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

    pub fn download_item(&mut self, entry: &FsEntry, dst_path: &Path) {
        if entry.file_type == FsEntryType::File {
            unsafe {
                let path = CString::new(path2bytes(&entry.path).unwrap()).unwrap();
                let mut fileinfo: libssh2_struct_stat = std::mem::uninitialized();

                let channel = libssh2_scp_recv2(self.session, path.as_ptr(), &mut fileinfo);

                if channel.is_null() {
                    println!("Failed to recv file: ");
                    print_error(self.session);
                    return;
                }
                let mut dest_file = std::fs::File::create(&dst_path).unwrap();

                let mut got = 0;
                let mut buffer: [u8; BUFFER_SIZE] = std::mem::uninitialized();
                let mut amount = BUFFER_SIZE as i64;

                while got < fileinfo.st_size {
                    if (fileinfo.st_size - got) < amount {
                        amount = (fileinfo.st_size - got) as i64;
                    }

                    let rc = libssh2_channel_read_ex(channel, 0, buffer.as_mut_ptr() as *mut _, amount as size_t) as i64;

                    if rc > 0 {
                        dest_file.write(&buffer[..rc as usize]).unwrap();
                    } else if rc < 0 {
                        println!("libssh2_channel_read() failed: {}", rc);
                        print_error(self.session);
                        break;
                    }
                    got += rc;
                }
                libssh2_channel_free(channel);
            }
        } else if entry.file_type == FsEntryType::Directory {
            std::fs::create_dir_all(&dst_path)
                .expect(format!("Failed to create directory: {:?}", &dst_path).as_str());
        }
    }

    pub fn run_cmd(&mut self, command: &str) -> Result<String, i32> {
        unsafe {
            let c_str = CString::new("session").unwrap();
            let channel_type = c_str.as_ptr() as *const c_char;
            let channel_type_len = "session".len() as c_uint;

            let channel = libssh2_channel_open_ex(
                self.session, channel_type,
                channel_type_len, LIBSSH2_CHANNEL_WINDOW_DEFAULT, LIBSSH2_CHANNEL_PACKET_DEFAULT,
                std::ptr::null_mut(), 0);
            if (channel as usize) == 0 {
                println!("erro on libssh2_channel_open_ex");
                print_error(self.session);
                return Err(-1);
            }

            let c_str = CString::new("exec").unwrap();
            let req_type = c_str.as_ptr() as *const c_char;
            let req_type_len = "exec".len() as c_uint;

            let c_str = CString::new(command).unwrap();
            let cmd = c_str.as_ptr() as *const c_char;
            let cmd_len = command.len() as c_uint;

            let rc = libssh2_channel_process_startup(channel, req_type, req_type_len, cmd, cmd_len);
            if rc != 0 {
                println!("Error on libssh2_channel_process_startup");
                print_error(self.session);
                return Err(-1);
            }

            let mut output = Vec::new();
            #[allow(deprecated)]
                let mut buffer: [u8; BUFFER_SIZE] = std::mem::uninitialized();//*mut c_char
            let buffer_len = buffer.len() as size_t;
            while
            {
                let rc = libssh2_channel_read_ex(channel, 0,
                                                 buffer.as_mut_ptr() as *mut _,
                                                 buffer_len) as usize;
                if rc > 0
                {
                    //println!("We read:");
                    output.append(&mut Vec::from(&buffer[0..rc]));
                } /*else {
                    if rc != LIBSSH2_ERROR_EAGAIN as usize {
                        println!("libssh2_channel_read returned  {}", rc);
                    }
                }*/
                rc > 0
            } {}
            let result = String::from_utf8_lossy(&output[..]).into_owned();
            libssh2_channel_free(channel);
            Ok(result)
        }
    }

    pub fn list_dir(&mut self, path: &Path) -> Vec<FsEntry> {
        let cmd = format!("unset LANG; find \"$(cd '{}'; pwd)\" -printf '%M|||%u|||%g|||%s|||%Ts|||%p|||%f|||%l\n'", path.display());
        let output = self.run_cmd(&cmd).unwrap();
        // Split output by \0
        let lines: Vec<&str> = output.as_str().split("\n").collect();
        let mut entries: Vec<FsEntry> = Vec::with_capacity(lines.len());
        //println!("lines {:?}", lines);
        let mut index = 0;
        for line in lines.iter() {
            // First line must always be ignored
            if index > 0 {
                let columns: Vec<&str> = line.split("|||").collect();
                //println!("columns {:?}", columns);
                let path = columns.get(5);
                if let Some(&p) = path {
                    if let Some(&pex) = columns.get(0) {
                        let mut file_type = pex.get(0..1).unwrap();
                        let mut is_link = false;
                        if file_type == "l" {
                            is_link = true;
                            if let Some(_link_info) = columns.get(7) {
                                let cmd = format!("unset LANG; find -L \"$(readlink -f {})\" -printf '%y'", p);
                                let link_file_type = self.run_cmd(&cmd).unwrap();
                                if link_file_type.contains("find:") {
                                    println!("link out: {} | cmd: {}", link_file_type, cmd);
                                } else {
                                    file_type = "-";
                                }
                            }
                        }
                        let entry = FsEntry {
                            path: p.parse().unwrap(),
                            file_type: match file_type {
                                "d" => FsEntryType::Directory,
                                _ => FsEntryType::File
                            },
                            is_link,
                        };
                        entries.push(entry);
                    }
                }
            }
            index += 1;
        }
        return entries;
    }

    pub fn disconnect(&mut self) {
        unsafe {
            let msg = CString::new("Normal Shutdown").unwrap();
            let lang = CString::new("").unwrap();
            libssh2_session_disconnect_ex(self.session, SSH_DISCONNECT_BY_APPLICATION, msg.as_ptr(), lang.as_ptr());
            libssh2_session_free(self.session);
        }
    }
}




