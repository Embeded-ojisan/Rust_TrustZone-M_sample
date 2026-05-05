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

type NsCallback = unsafe extern "cmse-nonsecure-call" fn();

static mut NS_CALLBACK: Option<NsCallback> = None;

#[no_mangle]
pub extern "cmse-nonsecure-entry" fn register_ns_callback(func: unsafe extern "C" fn()) {
    let func2: unsafe extern "cmse-nonsecure-call" fn() =
        unsafe { core::mem::transmute(func) };
    unsafe { NS_CALLBACK = Some(func2) };
}

#[no_mangle]
pub extern "cmse-nonsecure-entry" fn call_ns_function_from_secure() {
    unsafe {
        if let Some(f) = NS_CALLBACK {
            f();
        }
    }
}

//────────────────── SAU ──────────────────
pub unsafe fn init_sau() {
    use cortex_m::peripheral::sau::{Ctrl, Rbar, Rlar, Rnr};
    let sau = &*cortex_m::peripheral::SAU::PTR;

    // ── Region 0: NS Flash ──────────────────────────────────────────────────
    // AN521 Table 3-2 Row1: SSRAM1 NS alias = 0x0000_0000〜0x003F_FFFF
    // NS World が使う Flash 領域として 0x0010_0000〜0x0017_FFFF を割り当てる。
    const NS_FLASH_BASE:  u32 = 0x0010_0000;
    const NS_FLASH_LIMIT: u32 = 0x0017_FFFF;
    sau.rnr.write(Rnr(0));
    sau.rbar.write(Rbar(NS_FLASH_BASE));
    sau.rlar.write(Rlar((NS_FLASH_LIMIT & !0x1F) | 1));

    // ── Region 1: NS RAM ────────────────────────────────────────────────────
    // AN521 Table 3-4 Row8: SSRAM2 NS alias = 0x2800_0000〜0x281F_FFFF
    // NS World が使う RAM として 0x2810_0000〜0x2817_FFFF を割り当てる。
    const NS_RAM_BASE:  u32 = 0x2810_0000;
    const NS_RAM_LIMIT: u32 = 0x2817_FFFF;
    sau.rnr.write(Rnr(1));
    sau.rbar.write(Rbar(NS_RAM_BASE));
    sau.rlar.write(Rlar((NS_RAM_LIMIT & !0x1F) | 1));

    // ── Region 2: NSC (Non-Secure Callable) ─────────────────────────────────
    // AN521 Table 3-2 Row6: Secure Code 領域 (CODE NSC)
    // リンカスクリプトで .gnu.sgstubs を 0x1008_0000 に配置している想定。
    const NSC_BASE:  u32 = 0x1008_0000;
    const NSC_LIMIT: u32 = 0x1008_07FF;
    const RLAR_ENABLE: u32 = 1 << 0;
    const RLAR_NSC:    u32 = 1 << 1;
    sau.rnr.write(Rnr(2));
    sau.rbar.write(Rbar(NSC_BASE));
    sau.rlar.write(Rlar((NSC_LIMIT & !0x1F) | RLAR_ENABLE | RLAR_NSC));

    // ── Region 3: NS Peripheral ─────────────────────────────────────────────
    const NS_PERIPH_BASE:  u32 = 0x4000_0000;
    const NS_PERIPH_LIMIT: u32 = 0x4FFF_FFFF;
    sau.rnr.write(Rnr(3));
    sau.rbar.write(Rbar(NS_PERIPH_BASE));
    sau.rlar.write(Rlar((NS_PERIPH_LIMIT & !0x1F) | 1));

    // ── SAU 有効化 ──────────────────────────────────────────────────────────
    // ENABLE=1 (bit0), ALLNS=0 (bit1)
    sau.ctrl.write(Ctrl(0x1));
    core::arch::asm!("dsb sy; isb sy");

    for r in 0..4u32 {
        sau.rnr.write(Rnr(r));
        let rbar = sau.rbar.read().0;
        let rlar = sau.rlar.read().0;
        let _ = hprintln!(
            "[SAU] region={} RBAR=0x{:08X} RLAR=0x{:08X} EN={} NSC={}",
            r, rbar, rlar,
            rlar & 1,
            (rlar >> 1) & 1
        );
    }
}

//────────────────── MPC ──────────────────
// AN521 仕様より:
//   SSRAM1MPC: APB interface = 0x5800_7000
//   SSRAM2MPC: APB interface = 0x5800_8000
//
// BLK_CFG[3:0] からブロックサイズを動的に読み出す。
//   block_size = 1 << (BLK_CFG[3:0] + 5)
//   QEMU の AN521 実装では BLK_CFG=0x5 → block_size = 1 << 10 = 1024 bytes
//
// BLK_MAX: LUT の最終インデックス値（これを超えた書き込みは Bus Error）。
//
// LUT の構造:
//   1ワード(32bit) = 32ブロック分のセキュリティ設定
//   bit=0: Secure, bit=1: Non-secure
//   BLK_IDX=N のとき LUT[N] は block#(N*32)〜block#(N*32+31) に対応
//
// LUT インデックスの計算:
//   bytes_per_word = block_size * 32
//   lut_word_idx   = 物理アドレスオフセット / bytes_per_word
//
const MPC_SSRAM1_BASE: u32 = 0x5800_7000;
const MPC_SSRAM2_BASE: u32 = 0x5800_8000;

const BLK_MAX_OFFSET: u32 = 0x010;
const BLK_CFG_OFFSET: u32 = 0x014;
const BLK_IDX_OFFSET: u32 = 0x018;
const BLK_LUT_OFFSET: u32 = 0x01C;

/// MPC の LUT を NS に設定するヘルパー。
///
/// - `base_mpc`     : MPC の APB ベースアドレス
/// - `mem_bus_base` : このMPCが管理するメモリの NS バスアドレス先頭
/// - `ns_start`     : NS に設定したい範囲の先頭 NS バスアドレス
/// - `ns_end`       : NS に設定したい範囲の末尾 NS バスアドレス（inclusive）
/// - `label`        : ログ用ラベル
unsafe fn mpc_set_ns_range(
    base_mpc: u32,
    mem_bus_base: u32,
    ns_start: u32,
    ns_end: u32,
    label: &str,
) {
    // BLK_CFG からブロックサイズを動的取得（仕様値と実装値が乖離する場合があるため必須）
    let blk_cfg    = ptr::read_volatile((base_mpc + BLK_CFG_OFFSET) as *const u32);
    let block_size = 1u32 << ((blk_cfg & 0xF) + 5);

    // BLK_MAX: LUT の有効最終インデックス。これを超えた BLK_IDX 書き込みは Bus Error。
    let blk_max = ptr::read_volatile((base_mpc + BLK_MAX_OFFSET) as *const u32);

    // 物理オフセットから LUT ワードインデックスを計算
    let bytes_per_word = block_size * 32;
    let start_offset   = ns_start - mem_bus_base;
    let end_offset     = ns_end   - mem_bus_base;
    let start_word     = start_offset / bytes_per_word;
    let end_word       = end_offset   / bytes_per_word;

    let _ = hprintln!(
        "[MPC {}] block_size={} BLK_MAX={} bytes_per_word={} LUT[{}..={}]",
        label, block_size, blk_max, bytes_per_word, start_word, end_word
    );

    // BLK_MAX を超えるインデックスへの書き込みは Bus Error → 必ずガード
    if end_word > blk_max {
        let _ = hprintln!(
            "[MPC {}] ERROR: end_word={} > BLK_MAX={}, skipping",
            label, end_word, blk_max
        );
        return;
    }

    for idx in start_word..=end_word {
        ptr::write_volatile((base_mpc + BLK_IDX_OFFSET) as *mut u32, idx);
        ptr::write_volatile((base_mpc + BLK_LUT_OFFSET) as *mut u32, 0xFFFF_FFFF);
        // readback 確認
        ptr::write_volatile((base_mpc + BLK_IDX_OFFSET) as *mut u32, idx);
        let readback = ptr::read_volatile((base_mpc + BLK_LUT_OFFSET) as *const u32);
        if readback != 0xFFFF_FFFF {
            let _ = hprintln!(
                "[MPC {}] WARN: LUT[{}] readback=0x{:08X} (expected 0xFFFFFFFF)",
                label, idx, readback
            );
        }
    }

    let _ = hprintln!("[MPC {}] done", label);
}

pub unsafe fn init_mpc() {
    // ── NS Flash: 0x0010_0000〜0x0017_FFFF ──────────────────────────────────
    // SSRAM1 の NS バスアドレス先頭 = 0x0000_0000
    // SAU Region0 と範囲を一致させる。
    mpc_set_ns_range(
        MPC_SSRAM1_BASE,
        0x0000_0000,  // SSRAM1 NS バスアドレス先頭
        0x0010_0000,  // ns_start
        0x0017_FFFF,  // ns_end
        "SSRAM1 NS-Flash",
    );

    core::arch::asm!("dsb sy; isb sy");

    // ── NS RAM: 0x2810_0000〜0x2817_FFFF ────────────────────────────────────
    // SSRAM2 の NS バスアドレス先頭 = 0x2800_0000 (AN521 Table 3-4 Row8)
    // SAU Region1 と範囲を一致させる。
    mpc_set_ns_range(
        MPC_SSRAM2_BASE,
        0x2800_0000,  // SSRAM2 NS バスアドレス先頭
        0x2810_0000,  // ns_start
        0x2817_FFFF,  // ns_end
        "SSRAM2 NS-RAM",
    );

    core::arch::asm!("dsb sy; isb sy");

    let _ = hprintln!("[MPC] init done");

}

//────────────────── SPC ──────────────────
// AN521 Table 3-35: NSCCFG (0x5008_0014)
//   bit[0] CODENSC: CODE 領域 (0x1000_0000〜0x1FFF_FFFF) を NSC に設定
//   AN521 Table 3-2 Row6: Secure Code 領域は "CODE NSC" → CODENSC=1 が必要
const NSCCFG_ADDR: u32 = 0x5008_0014;

pub unsafe fn init_spc() {
    let nsc_cfg = NSCCFG_ADDR as *mut u32;
    let val = core::ptr::read_volatile(nsc_cfg);
    core::ptr::write_volatile(nsc_cfg, val | 0x1);
    let _ = hprintln!("[SPC] NSCCFG=0x{:08X}", core::ptr::read_volatile(nsc_cfg));
}

//────────────────── NS遷移 ──────────────────
#[inline(never)]
pub fn go_to_nonsecure() -> ! {
    const NONSECURE_VTOR:   u32 = 0x0010_0000;
    const NONSECURE_VTOR_S: u32 = 0x1010_0000;

    let msp_ns   = unsafe { *(NONSECURE_VTOR as *const u32) };
    let reset_ns = unsafe { *((NONSECURE_VTOR + 4) as *const u32) } & !1;
    
    let _ = hprintln!("[NS] msp_ns=0x{:08X} reset_ns=0x{:08X}", msp_ns, reset_ns);

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
    unsafe {
        core::ptr::write_volatile(0xE000_ED08 as *mut u32, 0x1000_0000);
    }

    let _ = hprintln!("Hello from secure! (mps2-an521)");
    unsafe { enable_faults(); }

    hprintln!("\nBefore initialize memory");
    check_memory();

    // 初期化順序: SPC → MPC → SAU
    unsafe { init_spc(); }
    unsafe { init_mpc(); }
    unsafe { init_sau(); }

    hprintln!("\nAfter initialize memory");
    check_memory();

    go_to_nonsecure();
}

fn check_memory() {
    unsafe {
        const SAU_RNR:  *mut u32 = 0xE000_EDD8 as *mut u32;
        const SAU_RBAR: *mut u32 = 0xE000_EDDC as *mut u32;
        const SAU_RLAR: *mut u32 = 0xE000_EDE0 as *mut u32;
        const SAU_CTRL: *mut u32 = 0xE000_EDD0 as *mut u32;

        let ctrl = ptr::read_volatile(SAU_CTRL);
        let _ = hprintln!("[SAU] CTRL=0x{:08X}", ctrl);

        for r in 0..4u32 {
            ptr::write_volatile(SAU_RNR, r);
            core::arch::asm!("dsb sy; isb sy");
            let rbar = ptr::read_volatile(SAU_RBAR);
            let rlar = ptr::read_volatile(SAU_RLAR);
            let _ = hprintln!(
                "[SAU] region={} RBAR=0x{:08X} RLAR=0x{:08X} EN={} NSC={}",
                r, rbar, rlar, rlar & 1, (rlar >> 1) & 1
            );
        }

        let addrs: &[u32] = &[
            0x0010_0000,
            0x0017_FFFF,
            0x1000_0000,
            0x1000_062A,
            0x1008_0000,
            0x1008_07FF,
            0x2810_0000,
            0x2817_FFFF,
            0x4000_0000,
        ];

        for &addr in addrs {
            let tt_result: u32;
            let ttt_result: u32;

            core::arch::asm!(
                "tt {result}, {addr}",
                result = out(reg) tt_result,
                addr   = in(reg)  addr,
            );
            core::arch::asm!(
                "ttt {result}, {addr}",
                result = out(reg) ttt_result,
                addr   = in(reg)  addr,
            );

            let s        = (tt_result >> 22) & 1;
            let srvalid  = (tt_result >> 17) & 1;
            let sregion  = (tt_result >>  8) & 0xFF;
            let irvalid  = (tt_result >> 23) & 1;
            let iregion  = (tt_result >> 24) & 0xFF;

            let ttt_s       = (ttt_result >> 22) & 1;
            let ttt_srvalid = (ttt_result >> 17) & 1;
            let ttt_sregion = (ttt_result >>  8) & 0xFF;

            let _ = hprintln!(
                "addr=0x{:08X}  TT: S={} SAUregion={} (valid={}) IDAUregion={} (valid={})  TTT: S={} SAUregion={} (valid={})",
                addr, s, sregion, srvalid, iregion, irvalid,
                ttt_s, ttt_sregion, ttt_srvalid
            );
        }
    }
}

//────────────────── panic ──────────────────
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}