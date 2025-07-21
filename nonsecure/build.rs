use std::env;

fn main() {
    // 変更検知
    println!("cargo:rerun-if-changed=memory.x");

    // クレート直下をリンク検索パスに追加
    println!(
        "cargo:rustc-link-search=native={}",
        env::var("CARGO_MANIFEST_DIR").unwrap()
    );

    // ここでは memory.x を -T では渡さない！
}