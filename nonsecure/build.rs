use std::env;

fn main() {
    // クレート直下をリンク検索パスに追加
    println!(
        "cargo:rustc-link-search=native={}",
        env::var("CARGO_MANIFEST_DIR").unwrap()
    );

}