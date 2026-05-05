#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::ptr;
use cortex_m_semihosting::hprintln;

extern "C" {
    static mut __sdata: u32;
    static mut __edata: u32;
    static     __sidata: u32;
    static mut __sbss:  u32;
    static mut __ebss:  u32;
    fn nonsecure_entry_function();
}

#[link_section = ".vector_table.reset_vector"]
#[no_mangle]
pub static EXCEPTIONS: [Option<unsafe extern "C" fn()>; 15] = unsafe {[
    Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(Reset)),
    None, // NMI
    Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(hard_fault_handler)),
    None, None, None, None, None, None, None, None,
    None, None, None, None,
]};

#[no_mangle]
pub unsafe extern "C" fn Reset() -> ! {
    let data_start = ptr::addr_of_mut!(__sdata);
    let data_end   = ptr::addr_of_mut!(__edata);
    let data_load  = ptr::addr_of!(__sidata);
    let count = data_end.offset_from(data_start) as usize;
    ptr::copy_nonoverlapping(data_load, data_start, count);

    let bss_start = ptr::addr_of_mut!(__sbss);
    let bss_end   = ptr::addr_of_mut!(__ebss);
    let count = bss_end.offset_from(bss_start) as usize;
    ptr::write_bytes(bss_start, 0, count);

    main()
}

#[no_mangle]
pub unsafe extern "C" fn hard_fault_handler() -> ! {
    hprintln!("[NS] HardFault");
    loop {}
}

fn main() -> ! {
    hprintln!("Hello from nonsecure!");
    unsafe { nonsecure_entry_function(); }
    loop {
        hprintln!("Hello from nonsecureloop!");
        for _ in 0..8_000_000 { cortex_m::asm::nop() }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}