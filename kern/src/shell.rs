use shim::io;
use shim::path::{Path, PathBuf};

use stack_vec::StackVec;

use pi::timer;
use pi::atags::Atags;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry};

use core::str;
use core::time::Duration;
use core::iter;

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
use crate::FILESYSTEM;

use alloc::vec::Vec;

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &str) -> ! {
    // Accept commands at most 512 bytes in length.
    let mut line_buf = [0u8;512];
    let mut line_buf = StackVec::new(&mut line_buf);
    let backspace = str::from_utf8(&[8, b' ', 8]).unwrap();

    kprintln!("Welcome to EOS :)   by LJR");
    loop {
        // Accept at most 64 arguments per command.
        let mut arg_buf = [""; 64];

        // Clear input line buf.
        line_buf.truncate(0);

        // Prefix before user entering command.
        kprint!("{}", prefix);

        // Keep reading byte until meet "\n" or "\r"
        // 1. Accept "\r" and "\n" as enter
        // 2. Accept backspace and delete (8 and 127) to erase a byte
        // 3. Ring the bell (7) for Unrecognized non-visible character
        loop {
            let byte = CONSOLE.lock().read_byte();
            match byte {
                32..=126 => {
                    CONSOLE.lock().write_byte(byte);
                    if let Err(_) = line_buf.push(byte) {
                        break;
                    }
                },
                8 | 127 => {
                    if line_buf.len() > 0 {
                        kprint!("{}", backspace);
                        line_buf.truncate(line_buf.len() - 1);
                    }
                },
                b'\n' | b'\r' => break,
                _ => CONSOLE.lock().write_byte(7),
            }
        }
        kprintln!("");

        let line = str::from_utf8(&line_buf).unwrap();
        match Command::parse(line, &mut arg_buf) {
            Ok(cmd) => {
                match cmd.path() {
                    "echo" => kprintln!("{}", line[cmd.args[0].len()..].trim_start()),
                    "print_atags" => {
                        for atag in Atags::get() {
                            kprintln!("{:#?}", atag);
                        }
                    },
                    "test_bin_alloc" => {
                        let mut v = Vec::new();
                        for i in 0..50 {
                            v.push(i);
                            kprintln!("{:?}", v);
                        }
                    },
                    _ => kprintln!("unknown command: {}", cmd.path()),
                }
            },
            Err(Error::TooManyArgs) => kprintln!("error: too many arguments"),
            Err(Error::Empty) => {},
        }
    }
}
