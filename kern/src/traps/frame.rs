use core::fmt;
use shim::const_assert_size;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TrapFrame {
    // FIXME: Fill me in.
    pub elr_elx: u64,
    spsr_elx: u64,
    sp_els: u64,
    tpidr_els: u64,
    q: [u128; 32], // q0...q31
    x: [u64; 31], // x0...x30(lr)
    xzr: u64, // for 16byte alignment purpose
}

const_assert_size!(TrapFrame, 800);

impl Default for TrapFrame {
    fn default() -> Self {
        TrapFrame {
            q: [0; 32],
            x: [0; 31],
            ..Default::default()
        }
    }
}
