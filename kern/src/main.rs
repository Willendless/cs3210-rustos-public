#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
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

// use console::kprintln;

use allocator::Allocator;
use fs::FileSystem;
use fs::sd::Sd;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

use pi::timer;
use pi::gpio::Gpio;
use core::time::Duration;

fn kmain() -> ! {
    let led = Gpio::new(16);
    let mut led = led.into_output();
    led.set();
    timer::spin_sleep(Duration::from_millis(5000));
    unsafe {
        ALLOCATOR.initialize();
    //     FILESYSTEM.initialize();
    }
    shell::shell("> ")
}
