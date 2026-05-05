#![no_std]
#![no_main]

#![feature(abi_cmse_nonsecure_call)]
#![feature(cmse_nonsecure_entry)]

use cortex_m_semihosting::hprintln;
use core::panic::PanicInfo;
use core::ptr;

unsafe extern "C" {
    static mut __sdata: u32;
    static mut __edata: u32;
    static     __sidata: u32;
    static mut __sbss:  u32;
    static mut __ebss:  u32;
}

//────────────────── ベクタテーブル ──────────────────
#[link_section = ".vector_table.reset_vector"]
#[no_mangle]
pub static EXCEPTIONS: [Option<unsafe extern "C" fn()>; 15] = unsafe {
    [
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(Reset)),
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(nmi_handler)),
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(hard_fault_handler)),
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(mem_manage_handler)),
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(bus_fault_handler)),
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(usage_fault_handler)),
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(secure_fault_handler)),
        None,
        None,
        None,
        None,
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(svcall_handler)),
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(debug_mon_handler)),
        None,
        Some(core::mem::transmute::<unsafe extern "C" fn() -> !, unsafe extern "C" fn()>(pend_sv_handler)),
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

//────────────────── 例外有効化 ──────────────────
unsafe fn enable_faults() {
    const SCB_SHCSR: *mut u32 = 0xE000_ED24 as *mut u32;
    let val = core::ptr::read_volatile(SCB_SHCSR);
    core::ptr::write_volatile(SCB_SHCSR, val | (1 << 16) | (1 << 17) | (1 << 18) | (1 << 19));
}

//────────────────── 例外ハンドラ ──────────────────
#[repr(C)]
pub struct ExceptionFrame {
    pub r0:   u32,
    pub r1:   u32,
    pub r2:   u32,
    pub r3:   u32,
    pub r12:  u32,
    pub lr:   u32,
    pub pc:   u32,
    pub xpsr: u32,
}

#[no_mangle]
pub unsafe extern "C" fn nmi_handler() -> ! {
    let _ = hprintln!("[EXCEPTION] NMI");
    loop {}
}

#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn hard_fault_handler() -> ! {
    core::arch::naked_asm!(
        "tst lr, #4",
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
    let cfsr = core::ptr::read_volatile(0xE000_ED28 as *const u32);
    let hfsr = core::ptr::read_volatile(0xE000_ED2C as *const u32);
    let _ = hprintln!(
        "[EXCEPTION] HardFault\n  PC=0x{:08X} LR=0x{:08X} xPSR=0x{:08X}\n  R0=0x{:08X} R1=0x{:08X} R2=0x{:08X} R3=0x{:08X}\n  CFSR=0x{:08X} HFSR=0x{:08X}",
        f.pc, f.lr, f.xpsr,
        f.r0, f.r1, f.r2, f.r3,
        cfsr, hfsr
    );
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn mem_manage_handler() -> ! {
    let cfsr  = core::ptr::read_volatile(0xE000_ED28 as *const u32);
    let mmfsr = (cfsr & 0xFF) as u8;
    let mmfar = core::ptr::read_volatile(0xE000_ED34 as *const u32);
    let mmfar_valid = (mmfsr & 0x80) != 0;
    let _ = hprintln!("[EXCEPTION] MemManageFault");
    let _ = hprintln!("  MMFSR = 0x{:02X}", mmfsr);
    if (mmfsr & 0x01) != 0 { let _ = hprintln!("  IACCVIOL  : instruction fetch violation"); }
    if (mmfsr & 0x02) != 0 { let _ = hprintln!("  DACCVIOL  : data access violation"); }
    if (mmfsr & 0x08) != 0 { let _ = hprintln!("  MUNSTKERR : unstack error on exception return"); }
    if (mmfsr & 0x10) != 0 { let _ = hprintln!("  MSTKERR   : stack error on exception entry"); }
    if (mmfsr & 0x20) != 0 { let _ = hprintln!("  MLSPERR   : lazy FP state save error"); }
    if mmfar_valid {
        let _ = hprintln!("  MMFAR     : 0x{:08X}  <-- fault address", mmfar);
    } else {
        let _ = hprintln!("  MMFAR     : invalid (MMARVALID=0)");
    }
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn bus_fault_handler() -> ! {
    let cfsr = core::ptr::read_volatile(0xE000_ED28 as *const u32);
    let bfsr = ((cfsr >> 8) & 0xFF) as u8;
    let bfar = core::ptr::read_volatile(0xE000_ED38 as *const u32);
    let bfar_valid = (bfsr & 0x80) != 0;
    let _ = hprintln!("[EXCEPTION] BusFault");
    let _ = hprintln!("  BFSR = 0x{:02X}", bfsr);
    if (bfsr & 0x01) != 0 { let _ = hprintln!("  IBUSERR    : instruction fetch bus error"); }
    if (bfsr & 0x02) != 0 { let _ = hprintln!("  PRECISERR  : precise data bus error"); }
    if (bfsr & 0x04) != 0 { let _ = hprintln!("  IMPRECISERR: imprecise data bus error"); }
    if (bfsr & 0x08) != 0 { let _ = hprintln!("  UNSTKERR   : unstack error on exception return"); }
    if (bfsr & 0x10) != 0 { let _ = hprintln!("  STKERR     : stack error on exception entry"); }
    if (bfsr & 0x20) != 0 { let _ = hprintln!("  LSPERR     : lazy FP state save error"); }
    if bfar_valid {
        let _ = hprintln!("  BFAR       : 0x{:08X}  <-- fault address", bfar);
    } else {
        let _ = hprintln!("  BFAR       : invalid (BFARVALID=0)");
    }
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn usage_fault_handler() -> ! {
    let cfsr = core::ptr::read_volatile(0xE000_ED28 as *const u32);
    let ufsr = ((cfsr >> 16) & 0xFFFF) as u16;
    let _ = hprintln!("[EXCEPTION] UsageFault");
    let _ = hprintln!("  UFSR = 0x{:04X}", ufsr);
    if (ufsr & 0x0001) != 0 { let _ = hprintln!("  UNDEFINSTR: undefined instruction"); }
    if (ufsr & 0x0002) != 0 { let _ = hprintln!("  INVSTATE  : invalid EPSR state"); }
    if (ufsr & 0x0004) != 0 { let _ = hprintln!("  INVPC     : invalid PC on exception return"); }
    if (ufsr & 0x0008) != 0 { let _ = hprintln!("  NOCP      : no coprocessor"); }
    if (ufsr & 0x0010) != 0 { let _ = hprintln!("  STKOF     : stack overflow"); }
    if (ufsr & 0x0100) != 0 { let _ = hprintln!("  UNALIGNED : unaligned access"); }
    if (ufsr & 0x0200) != 0 { let _ = hprintln!("  DIVBYZERO : divide by zero"); }
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn secure_fault_handler() -> ! {
    let sfsr = core::ptr::read_volatile(0xE000_EDE4 as *const u32);
    let sfar = core::ptr::read_volatile(0xE000_EDE8 as *const u32);
    let sfar_valid = (sfsr & 0x40) != 0;
    let _ = hprintln!("[EXCEPTION] SecureFault");
    let _ = hprintln!("  SFSR = 0x{:08X}", sfsr);
    if (sfsr & 0x01) != 0 { let _ = hprintln!("  INVEP  : invalid entry point (NS->S 非NSC経由)"); }
    if (sfsr & 0x02) != 0 { let _ = hprintln!("  INVIS  : invalid integrity signature"); }
    if (sfsr & 0x04) != 0 { let _ = hprintln!("  INVER  : invalid exception return"); }
    if (sfsr & 0x08) != 0 { let _ = hprintln!("  AUVIOL : attribution unit violation (SAU/IDAU)"); }
    if (sfsr & 0x10) != 0 { let _ = hprintln!("  INVTRAN: invalid transition (BLXNS/BXNS)"); }
    if (sfsr & 0x20) != 0 { let _ = hprintln!("  LSPERR : lazy state preservation error"); }
    if (sfsr & 0x80) != 0 { let _ = hprintln!("  LSERR  : lazy state error"); }
    if sfar_valid {
        let _ = hprintln!("  SFAR   : 0x{:08X}  <-- fault address", sfar);
    } else {
        let _ = hprintln!("  SFAR   : invalid (SFARVALID=0)");
    }
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

//────────────────── MPC ──────────────────
// AN521 DAI0521A Table 3-7 より
// 0x5800_7000: SSRAM1 MPC（Codeメモリ 0x00000000-, NS alias 0x00100000-）
// 0x5800_8000: SSRAM2 MPC（Expansion0  0x28000000-）
// 0x5800_5000: 未使用領域 → Bus Error（旧コードの誤り）
const MPC_SSRAM1_BASE: u32 = 0x5800_7000;
const MPC_SSRAM2_BASE: u32 = 0x5800_8000;

const CTRL_OFFSET:    u32 = 0x000;
const BLK_CFG_OFFSET: u32 = 0x014;
const BLK_IDX_OFFSET: u32 = 0x018;
const BLK_LUT_OFFSET: u32 = 0x01C;

// CTRLは触らない（デフォルト値のまま使う）
// 誤ったビット操作がAbortの原因のため削除

pub unsafe fn init_mpc() {
    // ── NS Flash: 0x00100000 〜 0x0017FFFF (512K) ──
    {
        let base_mpc = MPC_SSRAM1_BASE;
        let mem_base = 0x0010_0000u32;

        let blk_cfg    = ptr::read_volatile((base_mpc + BLK_CFG_OFFSET) as *const u32);
        let block_size = 1u32 << ((blk_cfg & 0xF) + 5);

        // CTRLは変更しない

        let ns_base  = 0x0010_0000u32;
        let ns_limit = 0x0017_FFFFu32;

        let start_index = (ns_base  - mem_base) / block_size / 32;
        let end_index   = (ns_limit + 1 - mem_base) / block_size / 32;

/*
        for index in start_index..end_index {
            ptr::write_volatile((base_mpc + BLK_IDX_OFFSET) as *mut u32, index);
            ptr::write_volatile((base_mpc + BLK_LUT_OFFSET) as *mut u32, 0xFFFF_FFFF);
        }
*/
    }

    // ── NS RAM: 0x28100000 〜 0x28107FFF (32K) ──
    {
        let base_mpc = MPC_SSRAM2_BASE;
        let mem_base = 0x2810_0000u32;

        let blk_cfg    = ptr::read_volatile((base_mpc + BLK_CFG_OFFSET) as *const u32);
        let block_size = 1u32 << ((blk_cfg & 0xF) + 5);

        let ns_base  = 0x2810_0000u32;
        let ns_limit = 0x2810_7FFFu32;

        let start_index = (ns_base  - mem_base) / block_size / 32;
        let end_index   = (ns_limit + 1 - mem_base) / block_size / 32;

/*
        for index in start_index..end_index {
            ptr::write_volatile((base_mpc + BLK_IDX_OFFSET) as *mut u32, index);
            ptr::write_volatile((base_mpc + BLK_LUT_OFFSET) as *mut u32, 0xFFFF_FFFF);
        }
*/
    }

    core::arch::asm!("dsb sy; isb sy");
    let _ = hprintln!("[MPC] init done");
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
    // VTOR_S を明示的に設定（デフォルト0x0のままだとフォルト時に誤ったテーブルを参照する）
    unsafe {
        core::ptr::write_volatile(0xE000_ED08 as *mut u32, 0x1000_0000);
    }

    let _ = hprintln!("Hello from secure! (mps2-an521)");
    unsafe { enable_faults(); }
    unsafe { init_sau(); }
    unsafe { init_mpc(); }

    go_to_nonsecure();
}

//────────────────── panic ──────────────────
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}