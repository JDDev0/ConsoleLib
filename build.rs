fn main() {
    let dst = cmake::build("consoleLib");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=ConsoleLib");

    if let Some(target_family) = std::env::var_os("CARGO_CFG_TARGET_FAMILY") {
        if target_family == "unix" {
            println!("cargo:rustc-link-lib=dylib=ncurses");
        }
    }
}
