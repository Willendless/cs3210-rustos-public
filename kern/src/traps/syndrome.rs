use aarch64::ESR_EL1;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Fault {
    AddressSize,
    Translation,
    AccessFlag,
    Permission,
    Alignment,
    TlbConflict,
    Other(u8),
}

impl From<u32> for Fault {
    fn from(val: u32) -> Fault {
        use self::Fault::*;

        // val[5:0]: I/DFSC, Instruction/Data Fault Status Code
        match  ESR_EL1::get_value(val as u64, ESR_EL1::ISS_IDABORT_IFSC) {
            0b000000..=0b000011 => AddressSize,
            0b000100..=0b000111 => Translation,
            0b001001..=0b001011 => AccessFlag,
            0b001101..=0b001111 => Permission,
            0b100001 => Alignment,
            0b110000 => TlbConflict,
            other => Other(other as u8),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Syndrome {
    Unknown,
    WfiWfe,
    SimdFp,
    IllegalExecutionState,
    Svc(u16),
    Hvc(u16),
    Smc(u16),
    MsrMrsSystem,
    InstructionAbort { kind: Fault, level: u8 },
    PCAlignmentFault,
    DataAbort { kind: Fault, level: u8 },
    SpAlignmentFault,
    TrappedFpu,
    SError,
    Breakpoint,
    Step,
    Watchpoint,
    Brk(u16),
    Other(u32),
}

/// Converts a raw syndrome value (ESR) into a `Syndrome` (ref: D1.10.4).
impl From<u32> for Syndrome {
    fn from(esr: u32) -> Syndrome {
        use self::Syndrome::*;

        let esr_64 = esr as u64;
        let hsvc_imm = ESR_EL1::get_value(esr_64, ESR_EL1::ISS_HSVC_IMM) as u16;
        let abort_level = ESR_EL1::get_value(esr_64, ESR_EL1::ISS_IDABORT_LEVEL) as u8;
        let brk_cmmt = ESR_EL1::get_value(esr_64, ESR_EL1::ISS_BRK_CMMT) as u16;

        match  ESR_EL1::get_value(esr_64, ESR_EL1::EC) {
            0b000000 => Unknown,
            0b000001 => WfiWfe,
            0b000111 => SimdFp,
            0b001110 => IllegalExecutionState,
            0b010101 => Svc(hsvc_imm),
            0b010110 => Hvc(hsvc_imm),
            0b010111 => Smc(hsvc_imm),
            0b011000 => MsrMrsSystem,
            0b100000..=0b100001 => InstructionAbort {
                kind: Fault::from(esr),
                level: abort_level,
            },
            0b100010 => PCAlignmentFault,
            0b100100..=0b100101 => DataAbort {
                kind: Fault::from(esr),
                level: abort_level,
            },
            0b100110 => SpAlignmentFault,
            0b101100 => TrappedFpu, // only handle AArch64
            0b101111 => SError,
            0b110000..=0b110001 => Breakpoint,
            0b110010..=0b110011 => Step,
            0b110100..=0b110101 => Watchpoint,
            0b111100 => Brk(brk_cmmt),
            _ => Other(esr),
        }
    }
}
