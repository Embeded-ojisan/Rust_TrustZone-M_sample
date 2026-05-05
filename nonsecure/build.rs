use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    println!("cargo:rustc-link-arg=-L{}", manifest_dir);
    println!("cargo:rustc-link-arg=-Tlink.ld");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let parent_parent_dir = out_dir.parent().unwrap().parent().unwrap().to_str().unwrap();
    println!("cargo:rustc-link-search=native={}", parent_parent_dir);

    println!("cargo:rerun-if-changed=link.ld");
}