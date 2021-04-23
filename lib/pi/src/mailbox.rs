use shim::io;
use crate::common::IO_BASE;

use volatile::prelude::*;
use volatile::{ReadVolatile, Volatile, WriteVolatile, Reserved};

const MAILBOX_REG_BASE: usize = IO_BASE + 0xB880;

// This bit is set in status register if no space to write to mailbox
const MAILBOX_FULL: u32 = 1 << 31;

// This bit is set in status register if nothing to read in maibox
const MAILBOX_EMPTY: u32 = 1 << 30;

#[derive(Copy, Clone)]
#[allow(non_snake_case)]
pub enum MailBoxChannel {
    PowerManagement,
    FrameBuffer,
    VirtualUart,
    Vchiq,
    Leds,
    Buttons,
    Touchscreen,
    Unused,
    PropertyArmToVc,
    PropertyVcToArm,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    READ: ReadVolatile<u32>,
    RESERVED: [Reserved<u32>; 3],
    PEEK: ReadVolatile<u32>,
    SENDER: ReadVolatile<u32>,
    STATUS: ReadVolatile<u32>,
    CONFIG: ReadVolatile<u32>,
    WRITE: WriteVolatile<u32>,
}

/// The Raspberry Pi ARM Mailbox.
pub struct Mailbox {
    registers: &'static mut Registers,
}

impl Mailbox {
    /// Returns a new instance of `Mailbox`.
    pub fn new() -> Mailbox {
        Mailbox {
            registers: unsafe { &mut *(MAILBOX_REG_BASE as *mut Registers) },
        }
    }

    pub fn read(&mut self, chan: MailBoxChannel) -> u32 {
        loop {
            while self.registers.STATUS.read() & MAILBOX_EMPTY > 0 {}
            let data = self.registers.READ.read();
            if data & 0xF == chan as u32 {
                return data & (!0xF);
            }
        }
    }

    pub fn write(&mut self, data: u32, chan: MailBoxChannel) {
        let val = (data & (!0xF)) | chan as u32;
        while self.registers.STATUS.read() & MAILBOX_FULL > 0 {}
        self.registers.WRITE.write(val);
    }
}
