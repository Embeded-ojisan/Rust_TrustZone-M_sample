fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // リンカスクリプトの探索パスを -L で渡す
    println!("cargo:rustc-link-arg=-L{}", manifest_dir);
    println!("cargo:rustc-link-arg=-Tlink.ld");
    println!("cargo:rerun-if-changed=link.ld");
}