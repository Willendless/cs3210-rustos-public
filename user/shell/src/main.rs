#![feature(asm)]
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[cfg(not(test))]
mod oom;
mod cr0;

extern crate alloc;

use kernel_api::{println, print};
use kernel_api::syscall::{fork, getpid, time, exit, read, brk};
use allocator::allocator::Allocator;
use alloc::string::String;

use shim::io;
use shim::path::{Path, PathBuf};
use shim::path::Component::*;

use core::str;
use stack_vec::StackVec;
use alloc::vec::Vec;

use kernel_api::syscall;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();

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
pub fn main() {
    let prefix = "> ";
    // Accept commands at most 512 bytes in length.
    let mut line_buf = [0u8;512];
    let mut line_buf = StackVec::new(&mut line_buf);
    let mut cwd = PathBuf::from("/");
    let mut exit = false;

    while !exit {
        // Clear input line buf.
        line_buf.truncate(0);
        // Prefix before user entering command.
        print!("({}) {}", cwd.to_str().unwrap(), prefix);
        // read command
        read_command(&mut line_buf);
        // forward to next line
        println!("");
        // run command
        let cmd = str::from_utf8(&line_buf).unwrap();
        parse_and_run(&mut cwd, cmd, &mut exit);
    }
}

fn read_command(buf: &mut StackVec<u8>) {
    let backspace: &'static str = str::from_utf8(&[8, b' ', 8]).unwrap();
    // Keep reading byte until meet "\n" or "\r"
    // 1. Accept "\r" and "\n" as enter
    // 2. Accept backspace and delete (8 and 127) to erase a byte
    // 3. Ring the bell (7) for Unrecognized non-visible character
    loop {
        let byte = syscall::read();

        match byte {
            32..=126 => {
                print!("{}", core::str::from_utf8(&[byte]).unwrap());
                if let Err(_) = buf.push(byte) {
                    break;
                }
            },
            8 | 127 => {
                if buf.len() > 0 {
                    print!("{}", backspace);
                    buf.truncate(buf.len() - 1);
                }
            },
            b'\n' | b'\r' => break,
            _ => println!(),
        }
    }
}

fn parse_and_run(cwd: &mut PathBuf, line: &str, exit: &mut bool) {
    // Accept at most 64 arguments per command
    let mut arg_buf = [""; 64];
    // Parse command line
    let cmd = match Command::parse(line, &mut arg_buf) {
        Ok(cmd) => cmd,
        Err(Error::TooManyArgs) => {
            println!("error: too many arguments");
            return;
        },
        Err(Error::Empty) => return,
    };

    match cmd.path() {
        "echo" => println!("{}", line[cmd.args[0].len()..].trim_start()),
        "yield" => syscall::r#yield(),
        "pwd" => cmd_pwd(),
        // "cd" => cmd_cd(cwd, &cmd),
        // "ls" => cmd_ls(cwd, &cmd),
        // "cat" => cmd_cat(cwd, &cmd),
        "sleep" => cmd_sleep(cwd, &cmd),
        "time" => cmd_time(cwd),
        "getpid" => cmd_getpid(cwd),
        "brk" => cmd_brk(cwd),
        // "name" => cmd_name(cwd),
        // "sp" => cmd_sp(cwd),
        // "exit" => *exit = true,
        _ => println!("unknown command: {}", cmd.path()),
    }
}

// Print the working directory.
fn cmd_pwd() {
    let mut buf = [0u8; 32];
    syscall::getcwd(&mut buf, 32);
    println!("{}", core::str::from_utf8(&buf).unwrap());
}

/// Change working directory.
///
/// # Format
///
/// ***cd \<directory\>***  
/// 
/// If there are no argument, working directory will be set to root directory.
// fn cmd_cd(cwd: &mut PathBuf, cmd: &Command) {
//     let path: PathBuf = if cmd.args.len() > 2 {
//         println!("sh: cd: too many arguments");
//         return;
//     } else if cmd.args.len() == 1 {
//         cwd.push("/");
//         return;
//     } else {
//         cmd.args[1].into()
//     };

//     // absolute directory
//     let dir = match parse_input_path(cwd, &path) {
//         Ok(dir) => dir,
//         Err(e) => {
//             println!("sh: cd: {}", e);
//             return;
//         }
//     };
//     match FILESYSTEM.open(&dir) {
//         Ok(entry) => match entry {
//             fat32::vfat::Entry::Dir(_) => cwd.push(dir),
//             fat32::vfat::Entry::File(_) => println!("sh: cd: {}: Not a directory", dir.display()),
//         },
//         _ => println!("sh: cd: invalid input"),
//     }
// }

// /// List the files in a directory.
// ///
// /// ## Format
// /// 
// /// ***ls [-a] [directory]***
// ///
// /// ## Options
// ///
// /// + `-a`: if passed in, hidden files are displayed, otherwise not displayed
// /// + `directory`: if not passed in, current working directory is displayed.
// ///
// /// ## Notice
// ///
// /// The arguments may be used together, but `-a` must be provided before `directory`
// fn cmd_ls(cwd: &PathBuf, cmd: &Command) {
//     let mut show_hidden = false;
//     let mut use_cwd = true;
//     let mut path = PathBuf::new();

//     // parse arguments
//     match cmd.args.len() {
//         3 => {
//             if cmd.args[1] != "-a" {
//                 println!("sh: ls: invalid option argument");
//                 return;
//             }
//             show_hidden = true;
//             use_cwd = false;
//             path = match parse_input_path(cwd, &cmd.args[2].into()) {
//                 Ok(path) => path,
//                 Err(e) => {
//                     println!("sh: ls: {}", e);
//                     return;
//                 } 
//             }
//         },
//         2 => {
//             show_hidden = if cmd.args[1] == "-a" { true } else { false };
//             use_cwd = if show_hidden { true } else {
//                 path = match parse_input_path(cwd, &cmd.args[1].into()) {
//                     Ok(path) => path,
//                     Err(e) => {
//                         println!("sh: ls: {}", e);
//                         return;
//                     } 
//                 };
//                 false
//             };
//         },
//         1 => {}, // use default value
//         _ => {
//             println!("sh: ls: too many arguments");
//             return;
//         },
//     }

//     // ls dir
//     let path = if use_cwd { cwd } else { &path };
//     match ls_path(path, show_hidden) {
//         Err(e) => println!("sh: ls: {}", e),
//         _ => {},
//     }
// }

// fn parse_input_path<'a, 'b>(cwd: &'a PathBuf, path: &'b PathBuf) -> Result<PathBuf, &'b str> {
//     // handle '.' and '..' in path
//     let mut dir = PathBuf::new();
//     for component in path.components() {
//         match component {
//             Prefix(_) => return Err("directory prefix not supported"),
//             RootDir => dir.push("/"),
//             CurDir => {},
//             ParentDir => {
//                 // precondition: if dir.file_name() is none,
//                 // dir must be "(../)*" or "/"
//                 if let None = dir.file_name() {
//                     // 1. dir is "(../)*", push ".."
//                     // 2. dir is root, do nothing
//                     match dir.has_root() {
//                         false => { dir.push(".."); },
//                         true => {},
//                     }
//                 } else {
//                     dir.pop();
//                 }
//             },
//             Normal(name) => dir.push(name),
//         }
//     }

//     // convert relative paths to absolute paths
//     let mut cwd_back = cwd.to_path_buf();
//     if !dir.has_root() {
//         for component in dir.components() {
//             match component {
//                 ParentDir => { cwd_back.pop(); } ,
//                 Normal(name) => cwd_back.push(name),
//                 _ => return Err("sd: cd: parse failed, should not reach here"),
//             };
//         }
//         dir = cwd_back;
//     }

//     Ok(dir)
// }

// fn ls_path<T: AsRef<Path>>(path: T, hidden: bool) -> io::Result<()> {
//     match FILESYSTEM.open_dir(path) {
//         Ok(dir) => {
//             for entry in dir.entries()? {
//                 if entry.is_hidden() && !hidden { continue; }
//                 println!("{}", entry);
//             }
//             Ok(())
//         },
//         Err(e) => Err(e),
//     }
// }

/// Concatenate files.
///
/// ## Formatter
///
/// ***cat <path..>***
///
/// Prints the contents of the files at the provided paths, one after the other.
/// At least one path argument is required.
// fn cmd_cat(cwd: &PathBuf, cmd: &Command) {
//     if cmd.args.len() == 1 {
//         println!("sh: cat: too less arguments");
//         return;
//     }

//     for arg in cmd.args[1..].iter() {
//         let path = match parse_input_path(cwd, &PathBuf::from(*arg)) {
//             Ok(path) => path,
//             Err(e) => {
//                 println!("sh: cat: {}", e);
//                 continue;
//             }
//         };
//         match print_file(&path) {
//             Err(e) => println!("sh: cat: {}", e),
//             _ => {},
//         };
//     }
// }

// fn print_file(path: &PathBuf) -> io::Result<()> {
//     let mut file = FILESYSTEM.open_file(path)?;
//     let mut buf = [0u8; 2048];

//     loop {
//         use shim::io::Read;
//         use shim::ioerr;

//         let read_size = file.read(&mut buf)?;
//         if read_size == 0 { break; }

//         let content = match str::from_utf8(&buf[..read_size]) {
//             Ok(s) => s,
//             Err(_) => return ioerr!(Other, "file contains invalid utf-8 character"),
//         };
//         print!("{}", content);
//     }

//     Ok(())
// }

/// Sleep ms.
///
/// sleep <ms>
///
fn cmd_sleep(_cwd: &PathBuf, cmd: &Command) {
    if cmd.args.len() > 2
        || cmd.args.len() == 1 {
        println!("sh: sleep: wrong number of arguments");
        return;
    }

    if let Ok(sleep_ms) = cmd.args[1].parse::<u64>() {
        use core::time::Duration;
        match syscall::sleep(Duration::from_millis(sleep_ms)) {
            Ok(elapsed_time) => println!("elapsed_time: {} ms", elapsed_time.as_millis()),
            Err(e) => println!("sh: sleep: error {:#?}", e),
        }
    } else {
        println!("sh: sleep: invalid argument");
    }
}

/// Get current time.
///
/// time
///
fn cmd_time(_cwd: &PathBuf) {
    println!("{:#?}", syscall::time());
}

/// Get current process's id.
///
/// process's id
///
fn cmd_getpid(_cwd: &PathBuf) {
    println!("id:{}", syscall::getpid());
}


/// Trap into kernel's debug shell
///
///
fn cmd_brk(_cwd: &PathBuf) {
    syscall::brk();
}
