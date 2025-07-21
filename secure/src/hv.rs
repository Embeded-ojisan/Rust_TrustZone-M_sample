use cortex_m::peripheral::{SAU, SYST};   // SCB を使わないなら外す

use cortex_m_semihosting::hprintln;

#[no_mangle]
static mut VM_TABLE: VmTable = VmTable { vms: [EMPTY_VM; MAX_VMS], current: 0 };

//────────────────── SAU / MPU (stub) ──────────────────
/// Secure Attribution Unit 初期化（Flash + SRAM を Non-Secure に）
pub unsafe fn init_sau_mpu() {
    use cortex_m::peripheral::sau::{Rnr, Rbar, Rlar, Ctrl};

/* 0x0020_0000 – 0x0027_FFFF : Non-Secure Flash 512 KiB */
const NS_FLASH_BASE  : u32 = 0x0020_0000;
const NS_FLASH_LIMIT : u32 = 0x0027_FFFF;

/* 0x2000_0000 – 0x2002_FFFF : Non-Secure SRAM 192 KiB (ゆとり) */
const NS_SRAM_BASE   : u32 = 0x2000_0000;
const NS_SRAM_LIMIT  : u32 = 0x2002_FFFF;

let sau = &*cortex_m::peripheral::SAU::PTR;
sau.rnr .write(Rnr (0));         // Flash
sau.rbar.write(Rbar(NS_FLASH_BASE));
sau.rlar.write(Rlar(NS_FLASH_LIMIT | 1));

sau.rnr .write(Rnr (1));         // SRAM
sau.rbar.write(Rbar(NS_SRAM_BASE));
sau.rlar.write(Rlar(NS_SRAM_LIMIT | 1));

sau.ctrl.write(Ctrl(1));
core::arch::asm!("dsb sy; isb sy");   // ← 忘れずに

    /* SAU ON */
    sau.ctrl.write(Ctrl(1));
}

#[inline(never)]
pub fn start_first_vm() -> ! {
    const nonsecure_VTOR: u32 = 0x0020_0000;
    let msp_ns   = unsafe { *(nonsecure_VTOR as *const u32) };
    let reset_ns = unsafe { *((nonsecure_VTOR + 4) as *const u32) } | 1; // Thumb

    /* VTOR_NS と MSP_NS を設定 */
    unsafe {
        core::ptr::write_volatile(0xE002_ED08 as *mut u32, nonsecure_VTOR);
        core::arch::asm!("dsb sy; isb sy");
        core::arch::asm!("msr MSP_NS, {0}", in(reg) msp_ns);
        core::arch::asm!(
            "bxns {entry}",
            entry = in(reg) reset_ns,
            options(noreturn)
        );
    }
}