mod frame;
mod syndrome;
mod syscall;

pub mod irq;
pub use self::frame::TrapFrame;

use pi::interrupt::{Controller, Interrupt};

use self::syndrome::Syndrome;
use self::syscall::handle_syscall;

use crate::shell;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Info {
    source: Source,
    kind: Kind,
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn handle_exception(info: Info, esr: u32, tf: &mut TrapFrame) {
    use aarch64::*;

    // kprintln!("exception happened: {:#?}", info);
    // kprintln!("current sp: 0x{:x}", SP.get());

    match info.kind {
        Kind::Synchronous => {
            use Syndrome::*;
            use aarch64::*;
            use crate::console::kprintln;
            
            kprintln!("sync exception captured in: 0x{:x}", unsafe { FAR_EL1.get() });
            match Syndrome::from(esr) {
                Brk(k) => {
                    kprintln!("brk exception: {:#?}", k);
                    shell::shell("debug > ");
                    tf.elr_elx += 4;
                },
                Svc(syscall_num) => {
                    // kprintln!("syscall: {}", syscall_num);
                    handle_syscall(syscall_num, tf);
                },
                other => {
                    kprintln!("Currently unhandled sync exception: {:#?}", other);
                }
            }
        },
        Kind::Irq => {
            let int_controller = Controller::new();
            for int in Interrupt::iter() {
                if int_controller.is_pending(*int) {
                    crate::IRQ.invoke(*int, tf);
                }
            }
        },
        Kind::Fiq => {},
        Kind::SError => {},
    }
}
