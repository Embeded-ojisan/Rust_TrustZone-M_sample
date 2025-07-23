#![no_std]
#![no_main]

use cortex_m_rt::entry;
use core::panic::PanicInfo;
use cortex_m_semihosting::hprintln;

extern "C" {
    fn nonsecure_entry_function();
}

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
