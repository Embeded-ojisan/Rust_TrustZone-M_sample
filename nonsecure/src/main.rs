#![no_std]
#![no_main]

#![feature(abi_cmse_nonsecure_call)]

use cortex_m_rt::entry;
use core::panic::PanicInfo;
use cortex_m_semihosting::hprintln;

use cortex_m_rt::exception;
use cortex_m_rt::ExceptionFrame;

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

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    hprintln!("--- HardFault in nonsecure ---");
    hprintln!("r0  = {:#010X}", ef.r0());
    hprintln!("r1  = {:#010X}", ef.r1());
    hprintln!("r2  = {:#010X}", ef.r2());
    hprintln!("r3  = {:#010X}", ef.r3());
    hprintln!("r12 = {:#010X}", ef.r12());
    hprintln!("lr  = {:#010X}", ef.lr());
    hprintln!("pc  = {:#010X}", ef.pc());
    hprintln!("xpsr= {:#010X}", ef.xpsr());

    let sfsr = core::ptr::read_volatile(0xE000_EDE4 as *const u32);
    hprintln!("SFSR={:08X}", sfsr);   // 0b0000_1000 → INVSTATE

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
