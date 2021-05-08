#![feature(asm)]
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[cfg(not(test))]
mod oom;
mod cr0;

extern crate alloc;

use kernel_api::println;
use kernel_api::syscall::{fork, getpid, time, exit, sleep};
use allocator::allocator::Allocator;
use alloc::string::String;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();

pub fn hanoi(n: i32, from: i32, to: i32, via: i32, steps: &mut u64) {
    if n > 0 {
      hanoi(n - 1, from, via, to, steps);
      *steps += 1;
      hanoi(n - 1, via, to, from, steps);
    }
  }

fn main() {
    let beg = time();
    let pid = getpid();
    println!("[{:02}] Started: {:?}", pid, beg);
    let mut steps = 0;
    hanoi(25, 1, 3, 2, &mut steps);
    let end = time();
    println!("[{:02}] Ended: {:?}", pid, end);
    println!("[{:02}] Result: {} ({:?})", pid, steps, end - beg);
}
