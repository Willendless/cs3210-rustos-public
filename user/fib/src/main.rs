#![feature(asm)]
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[cfg(not(test))]
mod oom;
mod cr0;

extern crate alloc;

use kernel_api::println;
use kernel_api::syscall::{fork, getpid, time, exit};
use allocator::allocator::Allocator;
use alloc::string::String;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();

fn fib(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fib(n - 1) + fib(n - 2),
    }
}

fn main() {
    match fork() {
        Ok(id) => {
            if id == 0 {
                println!("I am a forked child. My id is {}", getpid());
            } else {
                println!("Started...");
                let rtn = fib(40);
                println!("Ended: Result = {}", rtn);
            }
        },
        Err(e) => println!("Err: {:#?}", e)
    }
}
