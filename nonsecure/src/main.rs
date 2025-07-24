#![no_std]
#![no_main]

#![feature(abi_cmse_nonsecure_call)]

use cortex_m_rt::entry;
use core::panic::PanicInfo;
use cortex_m_semihosting::hprintln;

extern "C" {
    fn nonsecure_entry_function();
}

#[no_mangle]
#[link_section = ".ns_callable_fn"]
pub extern "C" fn hello_from_ns() {
    hprintln!("Hello from cmse_nonsecure_call!");
}

// ポインタを hello_from_ns に向ける（0x00200804 に配置させる）
#[used]
#[link_section = ".ns_callable_ptr"]
#[no_mangle]
pub static HELLO_FROM_NS_PTR: extern "cmse-nonsecure-call" fn() =
    unsafe { core::mem::transmute(hello_from_ns as extern "C" fn()) };

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[entry]
fn main() -> ! {
    hprintln!("Hello from nonsecure!");

    unsafe{ nonsecure_entry_function(); }

    loop {
        hprintln!("Hello from nonsecureloop!");
        for _ in 0..8_000_000 { cortex_m::asm::nop() } // dummy workload
    }
}
