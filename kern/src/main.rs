#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;
#[macro_use]
extern crate log;

pub mod allocator;
pub mod console;
pub mod fs;
pub mod logger;
pub mod mutex;
pub mod net;
pub mod param;
pub mod percore;
pub mod process;
pub mod shell;
pub mod traps;
pub mod vm;
pub mod gpu;

use allocator::Allocator;
use fs::FileSystem;
use net::uspi::Usb;
use net::GlobalEthernetDriver;
use process::GlobalScheduler;
use traps::irq::{Fiq, GlobalIrq};
use vm::VMManager;
use gpu::GlobalFrameBuffer;
use console::{kprintln};

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static USB: Usb = Usb::uninitialized();
pub static GLOABAL_IRQ: GlobalIrq = GlobalIrq::new();
pub static FIQ: Fiq = Fiq::new();
pub static ETHERNET: GlobalEthernetDriver = GlobalEthernetDriver::uninitialized();
pub static FRAMEBUFFER: GlobalFrameBuffer = GlobalFrameBuffer::uninitialized();

extern "C" {
    static __text_beg: u64;
    static __text_end: u64;
    static __bss_beg: u64;
    static __bss_end: u64;
}

unsafe fn kmain() -> ! {
    crate::logger::init_logger();

    // info!(
    //     "text beg: {:016x}, end: {:016x}",
    //     &__text_beg as *const _ as u64, &__text_end as *const _ as u64
    // );
    // info!(
    //     "bss  beg: {:016x}, end: {:016x}",
    //     &__bss_beg as *const _ as u64, &__bss_end as *const _ as u64
    // );

    // pi::timer::spin_sleep(core::time::Duration::from_millis(20000));
    ALLOCATOR.initialize();
    FRAMEBUFFER.initialize();
    FILESYSTEM.initialize();
    VMM.initialize();
    VMM.setup();
    SCHEDULER.initialize();
    SCHEDULER.start()
}

// use pi::timer;
// use pi::gpio::Gpio;
// use core::time::Duration;
// use pi::interrupt::Interrupt;
// use aarch64::*;

// unsafe fn kmain() -> ! {
//     ALLOCATOR.initialize();
//     FILESYSTEM.initialize();
//     IRQ.initialize();
//     VMM.initialize();
//     SCHEDULER.initialize();
//     SCHEDULER.start()
// }

// fn led_light(pin: u8) {
//     let led = Gpio::new(pin);
//     let mut led = led.into_output();
//     led.set();
// }

// fn welcome_output() {
//     info!("Welcome to EOS :) by LJR");
    // let led = pi::gpio::Gpio::new(16);
    // let mut led = led.into_output();
    // led.set();
    // TODO: output EOS logo
// }
