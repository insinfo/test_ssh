mod libssh2;
mod ssh2_rs;
mod libssh1;
mod thrussh_lib;


//#[tokio::main]
fn main() {
    // crate::ssh2_rs::run();
    // crate::libssh2::run();
    crate::libssh1::run();
    //crate::thrussh_lib::run();
}