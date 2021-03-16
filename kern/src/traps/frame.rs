use core::fmt;
use shim::const_assert_size;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    // FIXME: Fill me in.
    pub ttbr0_el1: u64,
    pub ttbr1_el1: u64,

    pub elr_elx: u64,
    pub spsr_elx: u64,
    pub sp_els: u64,
    pub tpidr_els: u64,
    pub q: [u128; 32], // q0...q31
    pub x: [u64; 31], // x0...x30(lr)
    pub xzr: u64, // for 16byte alignment purpose
}

const_assert_size!(TrapFrame, 816);
