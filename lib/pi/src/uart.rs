use core::fmt;
use core::time::Duration;

use shim::const_assert_size;
use shim::io;

use volatile::prelude::*;
use volatile::{ReadVolatile, Reserved, Volatile};

use crate::common::IO_BASE;
use crate::gpio::{Function, Gpio};
use crate::timer;

/// The base address for the `MU` registers.
const MU_REG_BASE: usize = IO_BASE + 0x215040;

/// The `AUXENB` register from page 9 of the BCM2837 documentation.
const AUX_ENABLES: *mut Volatile<u8> = (IO_BASE + 0x215004) as *mut Volatile<u8>;

/// Enum representing bit fields of the `AUX_MU_LSR_REG` register.
#[repr(u8)]
enum LsrStatus {
    DataReady = 1,
    TxAvailable = 1 << 5,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    // FIXME: Declare the "MU" registers from page 8.
    IO: Volatile<u8>,
    __r0: [Reserved<u8>; 3],
    IER: Volatile<u8>,
    __r1: [Reserved<u8>; 3],
    IIR: Volatile<u8>,
    __r2: [Reserved<u8>; 3],
    LCR: Volatile<u8>,
    __r3: [Reserved<u8>; 3],
    MCR: Volatile<u8>,
    __r4: [Reserved<u8>; 3],
    LSR: ReadVolatile<u8>,
    __r5: [Reserved<u8>; 3],
    MSR: ReadVolatile<u8>,
    __r6: [Reserved<u8>; 3],
    SCRATCH: Volatile<u8>,
    __r7: [Reserved<u8>; 3],
    CNTL: Volatile<u8>,
    __r8: [Reserved<u8>; 3],
    STAT: ReadVolatile<u32>,
    BAUD: Volatile<u16>,
    __r9: Reserved<u16>,
}

const_assert_size!(Registers, 0x7E21506C - 0x7E215040);

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<Duration>,
}

impl MiniUart {
    /// Initializes the mini UART by enabling it as an auxiliary peripheral,
    /// setting the data size to 8 bits, setting the BAUD rate to ~115200 (baud
    /// divider of 270), setting GPIO pins 14 and 15 to alternative function 5
    /// (TXD1/RDXD1), and finally enabling the UART transmitter and receiver.
    ///
    /// By default, reads will never time out. To set a read timeout, use
    /// `set_read_timeout()`.
    pub fn new() -> MiniUart {
        let registers = unsafe {
            // Enable the mini UART as an auxiliary device.
            (*AUX_ENABLES).or_mask(1);
            &mut *(MU_REG_BASE as *mut Registers)
        };

        // FIXME: Implement remaining mini UART initialization.
        // Set GPIO pins 14 and 15
        Gpio::new(14).into_alt(Function::Alt5);
        Gpio::new(15).into_alt(Function::Alt5);
        // Set data size to 8 bits
        registers.LCR.or_mask(0b11);
        // Set BAUD rate to ~115200
        registers.BAUD.write(270);
        // Enable the UART trasmitter and receiver
        registers.CNTL.or_mask(0b11);

        MiniUart {
            registers,
            timeout: None,
        }
    }

    /// Set the read timeout to `t` duration.
    pub fn set_read_timeout(&mut self, t: Duration) {
        self.timeout = Some(t);
    }

    /// Write the byte `byte`. This method blocks until there is space available
    /// in the output FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        while self.registers.LSR.has_mask(LsrStatus::TxAvailable as u8) == false {}
        self.registers.IO.write(byte);
    }

    /// Returns `true` if there is at least one byte ready to be read. If this
    /// method returns `true`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately. This method does not block.
    pub fn has_byte(&self) -> bool {
        self.registers.LSR.has_mask(LsrStatus::DataReady as u8)
    }

    /// Blocks until there is a byte ready to read. If a read timeout is set,
    /// this method blocks for at most that amount of time. Otherwise, this
    /// method blocks indefinitely until there is a byte to read.
    ///
    /// Returns `Ok(())` if a byte is ready to read. Returns `Err(())` if the
    /// timeout expired while waiting for a byte to be ready. If this method
    /// returns `Ok(())`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately.
    pub fn wait_for_byte(&self) -> Result<(), ()> {
        if let Some(timeout) = self.timeout {
            let cur = timer::current_time();
            let ddl = cur + timeout;
            while self.has_byte() == false {
                if timer::current_time() >= ddl {
                    return Err(());
                }
            }
            Ok(())
        } else {
            while self.has_byte() == false {}
            Ok(())
        }
    }

    /// Reads a byte. Blocks indefinitely until a byte is ready to be read.
    pub fn read_byte(&mut self) -> u8 {
        while self.has_byte() == false {}
        self.registers.IO.read()
    }
}

// FIXME: Implement `fmt::Write` for `MiniUart`. A b'\r' byte should be written
// before writing any b'\n' byte.

impl fmt::Write for MiniUart {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for ch in s.bytes() {
            if ch == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(ch);
        }
        Ok(())
    }
}

mod uart_io {
    use super::io;
    use super::ioerr;
    use super::MiniUart;
    use volatile::prelude::*;

    // FIXME: Implement `io::Read` and `io::Write` for `MiniUart`.
    //
    // The `io::Read::read()` implementation must respect the read timeout by
    // waiting at most that time for the _first byte_. It should not wait for
    // any additional bytes but _should_ read as many bytes as possible. If the
    // read times out, an error of kind `TimedOut` should be returned.
    //
    // The `io::Write::write()` method must write all of the requested bytes
    // before returning.
    impl io::Read for MiniUart {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if let Err(()) = self.wait_for_byte() {
                return ioerr!(TimedOut, "read from uart timedout");
            }
            let mut cnt = 0;
            while self.has_byte() && cnt < buf.len() {
                buf[cnt] = self.read_byte();
                cnt += 1;
            }
            Ok(cnt)
        }
    }

    impl io::Write for MiniUart {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            for ch in buf {
                self.write_byte(*ch);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
