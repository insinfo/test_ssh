fn main() {
    //antes execute vcpkg install libssh:x64-windows
    //isso para o compilador e o linker encontrar a libssh.lib
    println!(r"cargo:rustc-link-search=C:\src\vcpkg\installed\x64-windows\lib");
}