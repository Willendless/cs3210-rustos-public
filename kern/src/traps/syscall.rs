use alloc::boxed::Box;
use core::time::Duration;

use crate::console::CONSOLE;
use crate::process::State;
use crate::traps::TrapFrame;
use crate::SCHEDULER;
use kernel_api::*;
use pi::timer;

use crate::console::{kprintln,kprint};

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sys_sleep(ms: u32, tf: &mut TrapFrame) {
    let current_time = pi::timer::current_time();
    if let Some(awake_time) = current_time.checked_add(Duration::from_millis(ms.into())) {
        let is_awake = Box::new(move |p: &mut crate::process::Process| {
            if pi::timer::current_time() >= awake_time {
                p.trap_frame.x[0] = (pi::timer::current_time() - current_time).as_millis() as u64;
                p.trap_frame.x[7] = 1;
                true
            } else {
                false
            }
        });
        SCHEDULER.switch(State::Waiting(is_awake), tf);
    } else {
        kprintln!("timer overflow");
        // timer overflow
        tf.x[7] = 0;
    }
}

/// Returns current time.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns two
/// parameter:
///  - current time as seconds
///  - fractional part of the current time, in nanoseconds.
pub fn sys_time(tf: &mut TrapFrame) {
    let current_time = pi::timer::current_time();
    tf.x[0] = current_time.as_secs();
    tf.x[1] = current_time.subsec_nanos() as u64;
    tf.x[7] = 1;
}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    kprintln!("call sys_exit");
    SCHEDULER.switch(State::Dead, tf);
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {
    kprint!("{}", core::str::from_utf8(&[b]).unwrap());
    tf.x[7] = 1;
}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {
    tf.x[0] = aarch64::tid_el0();
    tf.x[7] = 1;
}

/// Fork current process. 
///
/// If success, current process will receive forked process's id
/// and the forked process will receive 0.
pub fn sys_fork(tf: &mut TrapFrame) {
    kprintln!("call sys_fork");
    match SCHEDULER.fork(tf) {
        Ok(id) => {
            tf.x[0] = id;
            tf.x[7] = 1;
        },
        Err(errnum) => tf.x[7] = errnum as u64
    }
}

/// Yield current CPU time interval.
pub fn sys_yield(tf: &mut TrapFrame) {
    SCHEDULER.switch(State::Ready, tf);
}

/// Returns a byte from CONSOLE.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: a byte from CONSOLE
pub fn sys_read(tf: &mut TrapFrame) {
    while !CONSOLE.lock().has_byte() {
        if (timer::current_time() >= SCHEDULER.get_next_tick_time()) {
            SCHEDULER.switch(State::Ready, tf);
        }
    }
    tf.x[0] = CONSOLE.lock().read_byte() as u64;
    tf.x[7] = 1;
}

/// Return current process's working directory.
///
/// This system call need two parameters: buf addr: VirtualAddr and size: usize
///
pub fn sys_getcwd(vaddr: u64, size: usize, tf: &mut TrapFrame) {
    // TODO: check virtualaddr
    kprintln!("getcwd: 0x{:x}, size: {}", vaddr, size);
    SCHEDULER.getcwd(vaddr, size);
    // TODO: adjust return value
    tf.x[7] = 1;
}

pub fn sys_open() {

}

pub fn sys_readfile(fd: u64, vaddr: u64, size: usize) {

}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    let num = num as usize;
    match num {
        NR_SLEEP => sys_sleep(tf.x[0] as u32, tf),
        NR_WRITE => sys_write(tf.x[0] as u8, tf),
        NR_EXIT => sys_exit(tf),
        NR_GETPID => sys_getpid(tf),
        NR_TIME => sys_time(tf),
        NR_FORK => sys_fork(tf),
        NR_YIELD => sys_yield(tf),
        NR_READ => sys_read(tf),
        NR_GETCWD => sys_getcwd(tf.x[0], tf.x[1] as usize, tf),
        _ => {}
    }
}
