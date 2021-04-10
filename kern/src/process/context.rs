use shim::{const_assert_eq, const_assert_size};

#[repr(C)]
#[derive(Default, Debug)]
pub struct Context {
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,
    pub lr: u64, // lr
    pub sp_el1: u64 // sp
}

const_assert_size!(Context, 104);
