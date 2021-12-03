use std::io::prelude::*;
use std::net::{TcpStream};
use ssh2::Session;
use std::path::{Path, PathBuf};
use test_ssh::{BUFFER_SIZE, FsEntry, FsEntryType};


pub fn run() {
    let dir_to_download = Path::new("/var/www/html/portalPmro");//
    let dest_dir_path = Path::new(r"C:\MyRustProjects\test_ssh\download");


    std::fs::create_dir_all(&dest_dir_path).expect(format!("Failed to create directory: {:?}", &dest_dir_path).as_str());


    //find / -name \*$'\n'\* -exec rm -rf {} \;
    // Connect to the local SSH server
    let tcp = TcpStream::connect("192.168.133.13:22").unwrap();
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    sess.userauth_password("isaque.neves", "Ins257257").unwrap();

    let start = std::time::Instant::now();
    let items_to_download = list_dir(&dir_to_download, &sess);
    println!("Time elapsed in list dir: {:?}", start.elapsed());

    let start = std::time::Instant::now();
    for item in items_to_download.iter() {
        download_item(&item, &sess, dir_to_download, dest_dir_path, None);
    }
    println!("Time elapsed in file transfer: {:?}", start.elapsed());

    println!("download of {:?} complete!", dir_to_download);
}

fn list_dir(path: &Path, sess: &Session) -> Vec<FsEntry> {
    let cmd = format!("unset LANG; find \"$(cd '{}'; pwd)\" -printf '%M|||%u|||%g|||%s|||%Ts|||%p|||%f|||%l\n'", path.display());
    let output = run_cmd(&cmd, &sess);
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
                            let link_file_type = run_cmd(&cmd, &sess);
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

fn download_item(entry: &FsEntry, sess: &Session, dir_to_download: &Path, local_path: &Path, _dst_name: Option<&str>) {
    /*let local_file_name = match dst_name {
        Some(n) => n,
        None => entry.path.file_name().unwrap().to_str().unwrap()
    };*/
    let path = &entry.path;
    //remove a parte inicial do caminho
    let mut name = PathBuf::from(path.strip_prefix(&dir_to_download).unwrap());
    name = local_path.join(name);

    //println!(" file name: {}", name.display());
    //println!("file name: {}", local_file_name);
    if entry.file_type == FsEntryType::File {
        if let Ok((mut src_file, _stat)) = sess.scp_recv(&path.as_path()) {
            println!("receiver file: {}", path.display());
            /*let dest_file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&name).unwrap();*/
            let mut dest_file = std::fs::File::create(&name).unwrap();

            //const CAP: usize = 8192;//8192;//1024 * 128;//4000000*8;
            //let mut writer = std::io::BufWriter::with_capacity(CAP, file);
            //let mut reader = std::io::BufReader::new(src_file);//with_capacity(CAP, src_file);
            //BUFFER_SIZE
            let mut buffer: [u8; BUFFER_SIZE] = unsafe {
                #[allow(deprecated)]
                std::mem::uninitialized()
            };
            loop {
                let num_read = src_file
                    .read(&mut buffer).expect("Could not read from remote");
                if num_read == 0 {
                    break;
                }
                dest_file.write_all(&buffer[0..num_read]).expect("Could not write");
            }
            /*loop {
                let length = {
                    let buffer = reader.fill_buf().unwrap();
                    // do stuff with buffer here
                    writer.write(buffer).expect("Unable to write file");
                    buffer.len()
                };
                if length == 0 {
                    break;
                }
                reader.consume(length);
            }*/
            //std::io::copy(&mut channel, &mut writer).unwrap();
        }
    } else if entry.file_type == FsEntryType::Directory {
        std::fs::create_dir_all(&name)
            .expect(format!("Failed to create directory: {:?}", &name).as_str());
    }
}

fn run_cmd(cmd: &str, sess: &Session) -> String {
    let mut channel = sess.channel_session().unwrap();
    //println!("cmd {}", cmd);
    channel.exec(&cmd).unwrap();
    let mut buffer = Vec::new();
    channel.read_to_end(&mut buffer).unwrap();
    channel.stderr().read_to_end(&mut buffer).unwrap();
    // println!("buffer {:?}", buffer);
    let output = String::from_utf8_lossy(&buffer).into_owned();
    //println!("output {}", output);
    channel.close().unwrap();
    return output;
}