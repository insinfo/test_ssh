#![allow(dead_code)]

use ssh::*;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use test_ssh::{FsEntry, FsEntryType};

const BUFFER_SIZE: usize = 1024 * 128;// 100 * 1024;  128 *

pub fn run() {
    let mut ssh = Libssh1::new("192.168.133.13").unwrap();
    ssh.connect("isaque.neves", "Ins257257");

    let dir_to_download = Path::new("/var/www/html/portalPmro");
    let dest_dir_path = Path::new(r"C:/MyRustProjects/test_ssh/download");
    std::fs::create_dir_all(&dest_dir_path).unwrap();

    let start = std::time::Instant::now();
    /* let items_to_download = ssh.list_dir(&dir_to_download);
     println!("Time elapsed in list dir: {:?}", start.elapsed());

     let start = std::time::Instant::now();
     for item in items_to_download.iter() {
         //remove a parte inicial do caminho
         let mut dst_path = PathBuf::from(&item.path.strip_prefix(&dir_to_download).unwrap());
         dst_path = dest_dir_path.join(dst_path);
         ssh.download_item(&item, &dst_path);
         println!("item {}", item.path.display());
     }*/

    ssh.download_dir_recursive(dir_to_download, dest_dir_path);
    println!("Time elapsed in file transfer: {:?}", start.elapsed());
    println!("download of {:?} complete!", dir_to_download);

    //println!("out: {}", ssh.run_cmd("ls -la").unwrap())
}

pub struct Libssh1 {
    pub session: Session,
}

impl Libssh1 {
    pub fn new(host: &str) -> Result<Libssh1, ()> {
        let mut session = Session::new().unwrap();
        session.set_host(host).unwrap();
        session.parse_config(None).unwrap();
        Ok(Libssh1 {
            session
        })
    }
    pub fn connect(&mut self, user: &str, pass: &str) {
        self.session.set_username(user).unwrap();
        self.session.connect().unwrap();
        self.session.userauth_password(pass).unwrap();
    }

    pub fn download_item(&mut self, entry: &FsEntry, dst_path: &Path) {
        if entry.file_type == FsEntryType::File {
            let mut scp = self.session.scp_new(READ, &entry.path).unwrap();

            scp.init().unwrap();
            loop {
                match scp.pull_request().unwrap() {
                    Request::NEWFILE => {
                        //let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
                        let mut buffer: Vec<u8> = vec!();
                        scp.accept_request().unwrap();
                        let src_file = scp.reader();//.read_to_end(&mut buf).unwrap();

                        let mut dest_file = std::fs::File::create(&dst_path).unwrap();

                        loop {
                            let num_read = src_file
                                .read_to_end(&mut buffer).expect("Could not read from remote");
                            if num_read == 0 {
                                break;
                            }
                            dest_file.write_all(&buffer).expect("Could not write");
                        }

                        break;
                    }
                    Request::WARNING => {
                        scp.deny_request().unwrap();
                        break;
                    }
                    _ => scp.deny_request().unwrap()
                }
            }
        } else if entry.file_type == FsEntryType::Directory {
            std::fs::create_dir_all(&dst_path)
                .expect(format!("Failed to create directory: {:?}", &dst_path).as_str());
        }
    }
    /// this is fast do download directory
    pub fn download_dir_recursive(&mut self, source_path: &Path, dst_path: &Path) {
        let mut scp = self.session.scp_new(READ | RECURSIVE, source_path).unwrap();

        match scp.init() {
            Ok(..) => {}
            Err(e) => { println!("error on init scp {}",e) }
        }

        let mut current_path = PathBuf::from(dst_path);

        loop {
            match scp.pull_request().unwrap() {
                Request::NEWFILE => {
                    let raw_filename = scp.request_get_filename().unwrap();
                    let temp_filename = String::from_utf8_lossy(raw_filename).to_owned();

                    let full_local_path_target = PathBuf::from(&current_path.join(&*temp_filename));
                    println!("full_local_path_target {:?}", full_local_path_target);

                    let mut dest_file = std::fs::File::create(full_local_path_target).unwrap();

                    scp.accept_request().unwrap();
                    let src_file = scp.reader();
                    let mut buffer: Vec<u8> = vec!();
                    src_file
                        .read_to_end(&mut buffer).expect("Could not read from remote");
                    dest_file.write_all(&buffer).expect("Could not write");

                    println!("NEWFILE");
                }
                Request::NEWDIR => {
                    //Um novo diretório será puxado
                    let raw_temp_dir_name = scp.request_get_filename().unwrap();
                    let temp_dir_name = String::from_utf8_lossy(raw_temp_dir_name).to_owned();
                    current_path = PathBuf::from(&current_path.join(&*temp_dir_name));
                    println!("current_path {:?}", current_path);
                    std::fs::create_dir_all(&current_path).unwrap();
                    scp.accept_request().unwrap();
                    println!("NEWDIR");
                }
                Request::EOF => {
                    println!("EOF");
                    break;
                }
                Request::ENDDIR => {
                    current_path = current_path.as_path().parent().unwrap().to_path_buf();
                    println!("ENDDIR {}", current_path.display());
                    //break;
                }
                Request::WARNING => {
                    println!("WARNING");
                    //scp.deny_request().unwrap();
                }
            }
        }
    }

    pub fn run_cmd(&mut self, command: &str) -> Result<String, i32> {
        let mut channel = self.session.channel_new().unwrap();
        channel.open_session().unwrap();
        channel.request_exec(command.as_bytes()).unwrap();
        channel.send_eof().unwrap();
        let mut buf = Vec::new();
        channel.stdout().read_to_end(&mut buf).unwrap();
        let out = String::from_utf8_lossy(&buf).into_owned();
        Ok(out)
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
}