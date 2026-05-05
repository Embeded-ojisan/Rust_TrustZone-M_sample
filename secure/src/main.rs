#![no_std]
#![no_main]

#![feature(abi_cmse_nonsecure_call)]
#![feature(cmse_nonsecure_entry)]

use cortex_m_semihosting::hprintln;
use core::panic::PanicInfo;
use core::ptr;

// .data / .bss のリンカシンボル
unsafe extern "C" {
    static mut __sdata: u32;
    static mut __edata: u32;
    static     __sidata: u32;
    static mut __sbss:  u32;
    static mut __ebss:  u32;
}

//────────────────── ベクタテーブル ──────────────────
/// Cortex-M33 例外ベクタテーブル
/// Reset後にCPUが参照する固定レイアウト
#[link_section = ".vector_table.reset_vector"]
#[no_mangle]
pub static EXCEPTIONS: [Option<unsafe extern "C" fn()>; 15] = unsafe {
    [
        // 0: Reset（Resetハンドラは別途先頭に配置）
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(Reset)),
        // 1: NMI
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(nmi_handler)),
        // 2: HardFault
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(hard_fault_handler)),
        // 3: MemManage
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(mem_manage_handler)),
        // 4: BusFault
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(bus_fault_handler)),
        // 5: UsageFault
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(usage_fault_handler)),
        // 6: SecureFault（M33 TrustZone固有）
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(secure_fault_handler)),
        // 7-10: 予約済み
        None,
        None,
        None,
        None,
        // 11: SVCall
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(svcall_handler)),
        // 12: DebugMon
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(debug_mon_handler)),
        // 13: 予約済み
        None,
        // 14: PendSV
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(pend_sv_handler)),
        // 15: SysTick
        // Some(sys_tick_handler),
    ]
};

//────────────────── Reset Handler ──────────────────
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

    main();
}

//────────────────── 例外ハンドラ ──────────────────

/// ExceptionFrame: スタックに自動退避されるレジスタ群
/// HardFault等でPCやLRを読み出すために使う
#[repr(C)]
pub struct ExceptionFrame {
    pub r0:   u32,
    pub r1:   u32,
    pub r2:   u32,
    pub r3:   u32,
    pub r12:  u32,
    pub lr:   u32,  // Link Register（呼び出し元）
    pub pc:   u32,  // 例外発生アドレス
    pub xpsr: u32,
}

#[no_mangle]
pub unsafe extern "C" fn nmi_handler() -> ! {
    let _ = hprintln!("[EXCEPTION] NMI");
    loop {}
}

/// HardFaultはスタックポインタ経由でExceptionFrameを読む
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn hard_fault_handler() -> ! {
    // MSPからExceptionFrameのアドレスをr0に渡してRust関数へ
    core::arch::naked_asm!(
        "tst lr, #4",           // EXC_RETURN bit2: 0=MSP, 1=PSP
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "b {hardfault_inner}",
        hardfault_inner = sym hard_fault_inner
    );
}

#[no_mangle]
pub unsafe extern "C" fn hard_fault_inner(frame: *const ExceptionFrame) -> ! {
    let f = &*frame;
    let _ = hprintln!(
        "[EXCEPTION] HardFault\n  PC=0x{:08X} LR=0x{:08X} xPSR=0x{:08X}\n  R0=0x{:08X} R1=0x{:08X} R2=0x{:08X} R3=0x{:08X}",
        f.pc, f.lr, f.xpsr,
        f.r0, f.r1, f.r2, f.r3
    );
    
    hprintln!("[EXCEPTION] HardFault");
    hprintln!("  PC=0x{:08X}", f.pc);
    hprintln!("  LR=0x{:08X}", f.lr);

    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn mem_manage_handler() -> ! {
    // MMFSR: SCB->CFSR[7:0]
    let cfsr = core::ptr::read_volatile(0xE000_ED28 as *const u32);
    let mmfsr = (cfsr & 0xFF) as u8;
    let mmfar = core::ptr::read_volatile(0xE000_ED34 as *const u32);
    let _ = hprintln!(
        "[EXCEPTION] MemManage MMFSR=0x{:02X} MMFAR=0x{:08X}",
        mmfsr, mmfar
    );
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn bus_fault_handler() -> ! {
    // BFSR: SCB->CFSR[15:8]
    let cfsr = core::ptr::read_volatile(0xE000_ED28 as *const u32);
    let bfsr = ((cfsr >> 8) & 0xFF) as u8;
    let bfar = core::ptr::read_volatile(0xE000_ED38 as *const u32);
    let _ = hprintln!(
        "[EXCEPTION] BusFault BFSR=0x{:02X} BFAR=0x{:08X}",
        bfsr, bfar
    );
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn usage_fault_handler() -> ! {
    // UFSR: SCB->CFSR[31:16]
    let cfsr = core::ptr::read_volatile(0xE000_ED28 as *const u32);
    let ufsr = ((cfsr >> 16) & 0xFFFF) as u16;
    let _ = hprintln!(
        "[EXCEPTION] UsageFault UFSR=0x{:04X}",
        ufsr
    );
    loop {}
}

/// SecureFault: M33 TrustZone固有
/// SAU違反・NSからSecure領域への不正アクセス等
#[no_mangle]
pub unsafe extern "C" fn secure_fault_handler() -> ! {
    // SFSR: 0xE000EDE4、SFAR: 0xE000EDE8
    let sfsr = core::ptr::read_volatile(0xE000_EDE4 as *const u32);
    let sfar = core::ptr::read_volatile(0xE000_EDE8 as *const u32);
    let _ = hprintln!(
        "[EXCEPTION] SecureFault SFSR=0x{:08X} SFAR=0x{:08X}",
        sfsr, sfar
    );
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn svcall_handler() -> ! {
    let _ = hprintln!("[EXCEPTION] SVCall");
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn debug_mon_handler() -> ! {
    let _ = hprintln!("[EXCEPTION] DebugMon");
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn pend_sv_handler() -> ! {
    let _ = hprintln!("[EXCEPTION] PendSV");
    loop {}
}

//────────────────── NSC エントリ ──────────────────
#[no_mangle]
pub extern "cmse-nonsecure-entry" fn nonsecure_entry_function() {
    let _ = hprintln!("Hello from secure (cmse-nonsecure-entry)!");
}

//────────────────── SAU ──────────────────
pub unsafe fn init_sau() {
    use cortex_m::peripheral::sau::{Ctrl, Rbar, Rlar, Rnr};
    let sau = &*cortex_m::peripheral::SAU::PTR;

    const NS_FLASH_BASE:  u32 = 0x0010_0000;
    const NS_FLASH_LIMIT: u32 = 0x0017_FFFF;
    sau.rnr.write(Rnr(0));
    sau.rbar.write(Rbar(NS_FLASH_BASE));
    sau.rlar.write(Rlar(NS_FLASH_LIMIT | 1));

    const NS_RAM_BASE:  u32 = 0x2810_0000;
    const NS_RAM_LIMIT: u32 = 0x2810_7FFF;
    sau.rnr.write(Rnr(1));
    sau.rbar.write(Rbar(NS_RAM_BASE));
    sau.rlar.write(Rlar(NS_RAM_LIMIT | 1));

    const NSC_BASE:    u32 = 0x1008_0800;
    const NSC_LIMIT:   u32 = 0x1008_0FFF;
    const RLAR_ENABLE: u32 = 1 << 0;
    const RLAR_NSC:    u32 = 1 << 1;
    sau.rnr.write(Rnr(2));
    sau.rbar.write(Rbar(NSC_BASE));
    sau.rlar.write(Rlar(NSC_LIMIT | RLAR_ENABLE | RLAR_NSC));

    const NS_PERIPH_BASE:  u32 = 0x4000_0000;
    const NS_PERIPH_LIMIT: u32 = 0x4FFF_FFFF;
    sau.rnr.write(Rnr(3));
    sau.rbar.write(Rbar(NS_PERIPH_BASE));
    sau.rlar.write(Rlar(NS_PERIPH_LIMIT | 1));

    sau.ctrl.write(Ctrl(1));
    core::arch::asm!("dsb sy; isb sy");
}

//────────────────── NS遷移 ──────────────────
#[inline(never)]
pub fn go_to_nonsecure() -> ! {
    const NONSECURE_VTOR: u32 = 0x0010_0000;

    let msp_ns   = unsafe { *(NONSECURE_VTOR as *const u32) };
    let reset_ns = unsafe { *((NONSECURE_VTOR + 4) as *const u32) } | 1;

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

//────────────────── main ──────────────────
fn main() -> ! {
    let _ = hprintln!("Hello from secure! (mps2-an521)");

    unsafe { init_sau(); }

    go_to_nonsecure();
}

//────────────────── panic ──────────────────
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}