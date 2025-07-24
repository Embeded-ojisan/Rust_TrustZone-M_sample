#![no_std]
#![no_main]

#![feature(abi_cmse_nonsecure_call)]
#![feature(cmse_nonsecure_entry)]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use cortex_m::peripheral::{SAU, SYST};
use core::panic::PanicInfo;

use cortex_m_rt::exception;
use cortex_m_rt::ExceptionFrame;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    hprintln!("--- HardFault ---");
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

//────────────────── SysTick ──────────────────
pub fn start_systick(ticks: u32) {
    assert!(ticks > 0 && ticks < 0x0100_0000);
    let syst = unsafe { &*SYST::PTR };
    unsafe {
        syst.rvr.write(ticks);
        syst.cvr.write(0);
        syst.csr.write((1<<0)|(1<<1)|(1<<2));
    }
}

#[export_name = "SysTick"]
pub unsafe extern "C" fn systick_handler() {
    // ★ 1. 関数ポインタが格納されているアドレスを読み取る
    let func_ptr_addr = 0x00200818 as *const usize;
    let func_addr = core::ptr::read_volatile(func_ptr_addr);

    hprintln!("NSC関数ポインタの中身: {:#010X}", func_addr);

    // ★ 2. ポインタを関数としてキャスト（cmse-nonsecure-call型に）
    let func: extern "cmse-nonsecure-call" fn() =
        core::mem::transmute::<usize, extern "cmse-nonsecure-call" fn()>(func_addr);

    hprintln!("NSC 関数ポインタから bxns でジャンプします");

    core::arch::asm!("dsb sy; isb sy");

   core::arch::asm!(
    "blxns {func}",
    func = in(reg) func as usize,
    // LR を clobber するので書いておくと親切
    lateout("lr") _,
    options(nostack)
    );


    hprintln!("NSC 関数から戻ってきました");
}


#[link_section = ".text_nonsecure_entry"]
#[no_mangle]
pub extern "cmse-nonsecure-entry" fn nonsecure_entry_function() {
    hprintln!("Hello from cmse-nonsecure-entry!");
}

fn is_valid_nonsecure_func(addr: *const u8) -> bool {
    let addr_val = addr as usize;
    (0x00200800..=0x002008FF).contains(&addr_val)
}

// flags = 0b001 (NS), 0b100 (read), 0b000 (no write)
const CMSE_NONSECURE: u32 = 1;
const CMSE_READ: u32 = 4;

//────────────────── SAU / MPU (stub) ──────────────────
/// Secure Attribution Unit 初期化（Flash + SRAM を Non-Secure に）
const SAU_RLAR_ENABLE: u32 = 1 << 0;
const SAU_RLAR_NSC: u32 = 1 << 1;

use cortex_m::peripheral::sau::Rnr;
use cortex_m::peripheral::sau::Rbar;
use cortex_m::peripheral::sau::Rlar;
use cortex_m::peripheral::sau::Ctrl;

pub unsafe fn init_sau_mpu() {
    let sau = &*cortex_m::peripheral::SAU::PTR;

    const SAU_RLAR_ENABLE: u32 = 1;
    const SAU_RLAR_NSC:    u32 = 1 << 1;

    #[inline(always)]
    const fn limit(addr: u32) -> u32 {
        // 32 byte 境界に丸める（bit[4:0]=11111 を強制）
        (addr & !0x1F) | SAU_RLAR_ENABLE
    }

    unsafe {
        // ── Region 0 : .ns_callable （NSC 付き）
        sau.rnr.write(Rnr(0));
        sau.rbar.write(Rbar(0x0020_0800));
        sau.rlar.write(Rlar(limit(0x0020_08FF) | SAU_RLAR_NSC));

        // ── Region 1 : 残りの Flash（NS, NSC なし）
        sau.rnr.write(Rnr(1));
        sau.rbar.write(Rbar(0x0020_0000));          // ★ 先頭を 0x0020_0000 に!
        sau.rlar.write(Rlar(limit(0x0020_07FF)));   // NSC フラグは付けない

        // ── Region 2 : SRAM（NS, NSC なし）
        sau.rnr.write(Rnr(2));
        sau.rbar.write(Rbar(0x0020_0900));
        sau.rlar.write(Rlar(limit(0x2002_FFFF)));      // ✘ NSC

        // ── Region 1 : 残りの Flash（NS, NSC なし）
        sau.rnr.write(Rnr(3));
        sau.rbar.write(Rbar(0x0020_0000));          // ★ 先頭を 0x0020_0000 に!
        sau.rlar.write(Rlar(limit(0x0027_FFFF)));   // NSC フラグは付けない

        sau.ctrl.write(Ctrl(1));   // SAU enable
        core::arch::asm!("dsb sy; isb sy");
    }
}

#[inline(never)]
pub fn go_to_nonsecure() -> ! {
    const NONSECURE_VTOR: u32 = 0x0020_0000;
    let msp_ns   = unsafe { *(NONSECURE_VTOR as *const u32) };
    let reset_ns = unsafe { *((NONSECURE_VTOR + 4) as *const u32) } | 1; // Thumb

    /* VTOR_NS と MSP_NS を設定 */
    unsafe {
        core::ptr::write_volatile(0xE002_ED08 as *mut u32, NONSECURE_VTOR);
        core::arch::asm!("dsb sy; isb sy");
        core::arch::asm!("msr MSP_NS, {0}", in(reg) msp_ns);
        core::arch::asm!(
            "bxns {entry}",
            entry = in(reg) reset_ns,
            options(noreturn)
        );
    }
}

fn print_sau_config() {
    let sau = unsafe { &*SAU::PTR };

    for region in 0..8 {
        unsafe {
            sau.rnr.write(cortex_m::peripheral::sau::Rnr(region));
            let rbar = sau.rbar.read().0;
            let rlar = sau.rlar.read().0;

            if rlar & 1 == 1 {
                let is_nsc = (rlar >> 1) & 1;
                hprintln!(
                    "SAU Region {}: {:08X} - {:08X} ({} NSC)",
                    region,
                    rbar,
                    rlar & 0xFFFFFFE0,
                    if is_nsc != 0 { "✔" } else { "✘" }
                );
            }
        }
    }
}

#[entry]
fn main() -> ! {
    hprintln!("Hello from secure!");

    unsafe {
        init_sau_mpu();
    }

    print_sau_config();

    hprintln!("Hello from secure2!");

    start_systick(64_000);   // 1 ms tick

    go_to_nonsecure();
}

