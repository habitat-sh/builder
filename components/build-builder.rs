// Inline common build behavior
include!("libbuild.rs");

fn main() { 
    builder::common(); 
    println!("cargo:rustc-link-lib=static=crypto");
    println!("cargo:rustc-link-lib=static=ssl");
    println!("cargo:rustc-link-lib=dylib=dl");
}
