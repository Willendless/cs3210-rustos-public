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

pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod shell;
pub mod param;
pub mod process;
pub mod traps;
pub mod vm;

use console::kprintln;

use allocator::Allocator;
use fs::FileSystem;
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static IRQ: Irq = Irq::uninitialized();

use pi::timer;
use pi::gpio::Gpio;
use core::time::Duration;

fn kmain() -> ! {
    led_light(16);
    timer::spin_sleep(Duration::from_millis(5000));
    welcome_output();
    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }
    shell::shell("> ")
}

fn led_light(pin: u8) {
    let led = Gpio::new(pin);
    let mut led = led.into_output();
    led.set();
}

fn welcome_output() {
    kprintln!("Welcome to EOS :) by LJR");
    // TODO: output EOS
}
