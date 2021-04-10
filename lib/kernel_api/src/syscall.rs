use core::fmt;
use core::fmt::Write;
use core::time::Duration;

use crate::*;

macro_rules! err_or {
    ($ecode:expr, $rtn:expr) => {{
        let e = OsError::from($ecode);
        if let OsError::Ok = e {
            Ok($rtn)
        } else {
            Err(e)
        }
    }};
}

pub fn sleep(span: Duration) -> OsResult<Duration> {
    if span.as_millis() > core::u64::MAX as u128 {
        panic!("too big!");
    }

    let ms = span.as_millis() as u64;
    let mut ecode: u64;
    let mut elapsed_ms: u64;

    unsafe {
        asm!("mov x0, $2
              svc $3
              mov $0, x0
              mov $1, x7"
             : "=r"(elapsed_ms), "=r"(ecode)
             : "r"(ms), "i"(NR_SLEEP)
             : "x0", "x7"
             : "volatile");
    }

    err_or!(ecode, Duration::from_millis(elapsed_ms))
}

pub fn time() -> Duration {
    let mut time_secs: u64;
    let mut subsec_nanos: u32;
    unsafe {
        asm!("svc $2
              mov $0, x0
              mov $1, x1"
              : "=r"(time_secs), "=r"(subsec_nanos)
              : "i"(NR_TIME)
              : "x0", "x1"
              : "volatile");
    }
    Duration::new(time_secs, subsec_nanos)
}

pub fn exit() -> ! {
    unsafe {
        asm!("svc $0"
            :: "i"(NR_EXIT)
            : "volatile");
    }
    unreachable!()
}

pub fn write(b: u8) {
    unsafe {
        asm!("mov x0, $0
              svc $1"
            :: "r"(b), "i"(NR_WRITE)
            : "x0"
            : "volatile"); 
    }
}

pub fn read() -> u8 {
    let b: u8;
    unsafe {
        asm!("svc $1
              mov $0, x0"
            : "=r"(b)
            : "i"(NR_READ)
            : "x0"
            : "volatile"); 
    }
    b
}

pub fn getpid() -> u64 {
    let pid: u64;
    unsafe {
        asm!("svc $1
              mov $0, x0"
            : "=r"(pid)
            : "i"(NR_GETPID)
            : "x0"
            : "volatile");
    }
    pid
}

pub fn fork() -> OsResult<u64> {
    let pid: u64;
    let ecode: u64;
    unsafe {
        asm!("svc $2
              mov $0, x0
              mov $1, x7"
            : "=r"(pid), "=r"(ecode)
            : "i"(NR_FORK)
            : "x0", "x7"
            : "volatile");
    }
    println!("child id: {}, ecode: {}", pid, ecode);
    err_or!(ecode, pid)
}

pub fn r#yield() {
    unsafe {
        asm!("svc $0"
            :: "i"(NR_YIELD)
            :: "volatile");
    }
}

pub fn getcwd(buf: &mut [u8], size: usize) {
    unsafe {
        asm!("mov x0, $0
              mov x1, $1
              svc $2"
            :: "r"(buf.as_ptr()), "r"(size), "i"(NR_GETCWD)
            :  "x0", "x1"
            :  "volatile")
    }
}

pub fn brk() {
    unsafe {
        asm!("brk 0":::: "volatile");
    }
}

struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            write(b);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::syscall::vprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
 () => (print!("\n"));
    ($($arg:tt)*) => ({
        $crate::syscall::vprint(format_args!($($arg)*));
        $crate::print!("\n");
    })
}

pub fn vprint(args: fmt::Arguments) {
    let mut c = Console;
    c.write_fmt(args).unwrap();
}
