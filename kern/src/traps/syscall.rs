use alloc::boxed::Box;
use core::time::Duration;

use smoltcp::wire::{IpAddress, IpEndpoint};

use crate::console::{kprint, CONSOLE, kprintln};
use crate::param::USER_IMG_BASE;
use crate::process::State;
use crate::traps::TrapFrame;
use crate::{ETHERNET, SCHEDULER};

use pi::timer;
use kernel_api::*;

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
            // kprintln!("current time: {:#?}, awake_time: {:#?}", pi::timer::current_time(), awake_time);
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

/// Kills the current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    // TODO: may need to modify
    SCHEDULER.switch(State::Dead, tf);
}

/// Writes to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {
    kprint!("{}", core::str::from_utf8(&[b]).unwrap());
    tf.x[7] = 1;
}

/// Returns the current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {
    tf.x[0] = aarch64::tid_el0();
    tf.x[7] = 1;
}


pub fn sys_getpriority(tf: &mut TrapFrame) {
    tf.x[0] = SCHEDULER.get_priority();
    tf.x[7] = 1;
}

/// Fork current process. 
///
/// If success, current process will receive forked process's id
/// and the forked process will receive 0.
pub fn sys_fork(tf: &mut TrapFrame) {
    info!("syscall: sys_fork called");
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

/// Creates a socket and saves the socket handle in the current process's
/// socket list.
///
/// This function does neither take any parameter nor return anything,
/// except the usual return code that indicates successful syscall execution.
pub fn sys_sock_create(tf: &mut TrapFrame) {
    // Lab 5 2.D
    unimplemented!("sys_sock_create")
}

/// Returns the status of a socket.
///
/// This system call takes a socket descriptor as the first parameter.
///
/// In addition to the usual status value, this system call returns four boolean
/// values that describes the status of the queried socket.
///
/// - x0: is_active
/// - x1: is_listening
/// - x2: can_send
/// - x3: can_recv
///
/// # Errors
/// This function returns `OsError::InvalidSocket` if a socket that corresponds
/// to the provided descriptor is not found.
pub fn sys_sock_status(sock_idx: usize, tf: &mut TrapFrame) {
    // Lab 5 2.D
    unimplemented!("sys_sock_status")
}

/// Connects a local ephemeral port to a remote IP endpoint with a socket.
///
/// This system call takes a socket descriptor as the first parameter, the IP
/// of the remote endpoint as the second paramter in big endian, and the port
/// number of the remote endpoint as the third parameter.
///
/// `handle_syscall` should read the value of registers and create a struct that
/// implements `Into<IpEndpoint>` when calling this function.
///
/// It only returns the usual status value.
///
/// # Errors
/// This function can return following errors:
///
/// - `OsError::NoEntry`: Fails to allocate an ephemeral port
/// - `OsError::InvalidSocket`: Cannot find a socket that corresponds to the provided descriptor.
/// - `OsError::IllegalSocketOperation`: `connect()` returned `smoltcp::Error::Illegal`.
/// - `OsError::BadAddress`: `connect()` returned `smoltcp::Error::Unaddressable`.
/// - `OsError::Unknown`: All the other errors from calling `connect()`.
pub fn sys_sock_connect(
    sock_idx: usize,
    remote_endpoint: impl Into<IpEndpoint>,
    tf: &mut TrapFrame,
) {
    // Lab 5 2.D
    unimplemented!("sys_sock_connect")
}

/// Listens on a local port for an inbound connection.
///
/// This system call takes a socket descriptor as the first parameter and the
/// local ports to listen on as the second parameter.
///
/// It only returns the usual status value.
///
/// # Errors
/// This function can return following errors:
///
/// - `OsError::InvalidSocket`: Cannot find a socket that corresponds to the provided descriptor.
/// - `OsError::IllegalSocketOperation`: `listen()` returned `smoltcp::Error::Illegal`.
/// - `OsError::BadAddress`: `listen()` returned `smoltcp::Error::Unaddressable`.
/// - `OsError::Unknown`: All the other errors from calling `listen()`.
pub fn sys_sock_listen(sock_idx: usize, local_port: u16, tf: &mut TrapFrame) {
    // Lab 5 2.D
    unimplemented!("sys_sock_listen")
}

/// Returns a slice from a virtual address and a legnth.
///
/// # Errors
/// This functions returns `Err(OsError::BadAddress)` if the slice is not entirely
/// in userspace.
unsafe fn to_user_slice<'a>(va: usize, len: usize) -> OsResult<&'a [u8]> {
    let overflow = va.checked_add(len).is_none();
    if va >= USER_IMG_BASE && !overflow {
        Ok(core::slice::from_raw_parts(va as *const u8, len))
    } else {
        Err(OsError::BadAddress)
    }
}

/// Returns a mutable slice from a virtual address and a legnth.
///
/// # Errors
/// This functions returns `Err(OsError::BadAddress)` if the slice is not entirely
/// in userspace.
unsafe fn to_user_slice_mut<'a>(va: usize, len: usize) -> OsResult<&'a mut [u8]> {
    let overflow = va.checked_add(len).is_none();
    if va >= USER_IMG_BASE && !overflow {
        Ok(core::slice::from_raw_parts_mut(va as *mut u8, len))
    } else {
        Err(OsError::BadAddress)
    }
}

/// Sends data with a connected socket.
///
/// This system call takes a socket descriptor as the first parameter, the
/// address of the buffer as the second parameter, and the length of the buffer
/// as the third parameter.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the number of bytes sent.
///
/// # Errors
/// This function can return following errors:
///
/// - `OsError::InvalidSocket`: Cannot find a socket that corresponds to the provided descriptor.
/// - `OsError::BadAddress`: The address and the length pair does not form a valid userspace slice.
/// - `OsError::IllegalSocketOperation`: `send_slice()` returned `smoltcp::Error::Illegal`.
/// - `OsError::Unknown`: All the other errors from smoltcp.
pub fn sys_sock_send(sock_idx: usize, va: usize, len: usize, tf: &mut TrapFrame) {
    // Lab 5 2.D
    unimplemented!("sys_sock_send")
}

/// Receives data from a connected socket.
///
/// This system call takes a socket descriptor as the first parameter, the
/// address of the buffer as the second parameter, and the length of the buffer
/// as the third parameter.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the number of bytes read.
///
/// # Errors
/// This function can return following errors:
///
/// - `OsError::InvalidSocket`: Cannot find a socket that corresponds to the provided descriptor.
/// - `OsError::BadAddress`: The address and the length pair does not form a valid userspace slice.
/// - `OsError::IllegalSocketOperation`: `recv_slice()` returned `smoltcp::Error::Illegal`.
/// - `OsError::Unknown`: All the other errors from smoltcp.
pub fn sys_sock_recv(sock_idx: usize, va: usize, len: usize, tf: &mut TrapFrame) {
    // Lab 5 2.D
    unimplemented!("sys_sock_recv")
}

/// Writes a UTF-8 string to the console.
///
/// This system call takes the address of the buffer as the first parameter and
/// the length of the buffer as the second parameter.
///
/// In addition to the usual status value, this system call returns the length
/// of the UTF-8 message.
///
/// # Errors
/// This function can return following errors:
///
/// - `OsError::BadAddress`: The address and the length pair does not form a valid userspace slice.
/// - `OsError::InvalidArgument`: The provided buffer is not UTF-8 encoded.
pub fn sys_write_str(va: usize, len: usize, tf: &mut TrapFrame) {
    let result = unsafe { to_user_slice(va, len) }
        .and_then(|slice| core::str::from_utf8(slice).map_err(|_| OsError::InvalidArgument));

    match result {
        Ok(msg) => {
            kprint!("{}", msg);

            tf.x[0] = msg.len() as u64;
            tf.x[7] = OsError::Ok as u64;
        }
        Err(e) => {
            tf.x[7] = e as u64;
        }
    }
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
        NR_WRITE_STR => sys_write_str(tf.x[0] as usize, tf.x[1] as usize, tf),
        NR_GETPRIORITY => sys_getpriority(tf),
        _ => {
            kprintln!("unimplemented syscall");
            unreachable!()
        }
    }
}
