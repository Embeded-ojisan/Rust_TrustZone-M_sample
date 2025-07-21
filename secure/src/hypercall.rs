//! Secure World から Non‑Secure へ公開する簡易ハイパコール実装。

/// Veneer (C) 側から参照されるシンボル。
/// 今は常に "nonsecure" として 0 を返すだけ。
#[no_mangle]
pub extern "C" fn secure_current_vm() -> u32 {
    0
}