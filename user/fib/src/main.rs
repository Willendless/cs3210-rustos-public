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
    let beg = time();
    let pid = getpid();
    println!("[{:02}] Started: {:?}", pid, beg);
    let rtn = fib(40);
    let end = time();
    println!("[{:02}] Ended: {:?}", pid, end);
    println!("[{:02}] Result: {} ({:?})", pid, rtn, end - beg);
}
