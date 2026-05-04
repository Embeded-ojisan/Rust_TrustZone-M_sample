#![no_std]
#![no_main]

#![feature(abi_cmse_nonsecure_call)]
#![feature(cmse_nonsecure_entry)]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/*
 * Non-secure から呼べる Secure エントリ（NSC 経由のデモ）
 * - secure 側リンク時に --cmse-implib で veneers.o が生成され、
 *   nonsecure 側がそれをリンクして呼べるようになる想定
 */
//#[link_section = ".text_nonsecure_entry"]
#[no_mangle]
pub extern "cmse-nonsecure-entry" fn nonsecure_entry_function() {
    // NS から呼ばれたらここが動く
    let _ = hprintln!("Hello from secure (cmse-nonsecure-entry)!");
}

//────────────────── SAU (MPS2-AN521) ──────────────────
/// Secure Attribution Unit 初期化：
/// - Non-secure Flash: 0x0010_0000 .. 0x0017_FFFF (512KB)
/// - Non-secure RAM  : 0x2810_0000 .. 0x2810_7FFF (32KB)
/// - Non-secure Periph window: 0x4000_0000 .. 0x4FFF_FFFF
/// - NSC veneer      : 0x1008_0800 .. 0x1008_0FFF (例: 2KB〜4KB程度)
pub unsafe fn init_sau() {
    use cortex_m::peripheral::sau::{Ctrl, Rbar, Rlar, Rnr};
    let sau = &*cortex_m::peripheral::SAU::PTR;

    // Region 0: Non-secure Flash
    const NS_FLASH_BASE:  u32 = 0x0010_0000;
    const NS_FLASH_LIMIT: u32 = 0x0017_FFFF;

    sau.rnr.write(Rnr(0));
    sau.rbar.write(Rbar(NS_FLASH_BASE));
    sau.rlar.write(Rlar(NS_FLASH_LIMIT | 1)); // ENABLE=1

    // Region 1: Non-secure RAM
    const NS_RAM_BASE:  u32 = 0x2810_0000;
    const NS_RAM_LIMIT: u32 = 0x2810_7FFF;

    sau.rnr.write(Rnr(1));
    sau.rbar.write(Rbar(NS_RAM_BASE));
    sau.rlar.write(Rlar(NS_RAM_LIMIT | 1));

    // Region 2: NSC veneer (Secure flash 内)
    // SAU_RLAR.NSC は bit1
    const NSC_BASE:  u32 = 0x1008_0800;
    const NSC_LIMIT: u32 = 0x1008_0FFF;
    const RLAR_ENABLE: u32 = 1 << 0;
    const RLAR_NSC:    u32 = 1 << 1;

    sau.rnr.write(Rnr(2));
    sau.rbar.write(Rbar(NSC_BASE));
    sau.rlar.write(Rlar(NSC_LIMIT | RLAR_ENABLE | RLAR_NSC));

    // Region 3: Non-secure Peripheral window
    const NS_PERIPH_BASE:  u32 = 0x4000_0000;
    const NS_PERIPH_LIMIT: u32 = 0x4FFF_FFFF;

    sau.rnr.write(Rnr(3));
    sau.rbar.write(Rbar(NS_PERIPH_BASE));
    sau.rlar.write(Rlar(NS_PERIPH_LIMIT | 1));

    // SAU enable + barrier
    sau.ctrl.write(Ctrl(1));
    core::arch::asm!("dsb sy; isb sy");
}

#[inline(never)]
pub fn go_to_nonsecure() -> ! {
    // Non-secure image base (aligned with your zephyr.map)
    const NONSECURE_VTOR: u32 = 0x0010_0000;

    let msp_ns   = unsafe { *(NONSECURE_VTOR as *const u32) };
    let reset_ns = unsafe { *((NONSECURE_VTOR + 4) as *const u32) } | 1; // Thumb

    unsafe {
        // VTOR_NS (Secure から Non-secure SCS alias へ書く)
        core::ptr::write_volatile(0xE002_ED08 as *mut u32, NONSECURE_VTOR);
        core::arch::asm!("dsb sy; isb sy");

        // MSP_NS
        core::arch::asm!("msr MSP_NS, {0}", in(reg) msp_ns);

        // branch to NS Reset
        core::arch::asm!(
            "bxns {entry}",
            entry = in(reg) reset_ns,
            options(noreturn)
        );
    }
}

#[entry]
fn main() -> ! {
    let _ = hprintln!("Hello from secure! (mps2-an521)");

    unsafe {
        init_sau();
    }

    go_to_nonsecure();
}
