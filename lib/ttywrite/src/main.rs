mod parsers;

use serial;
use structopt;
use structopt_derive::StructOpt;
use xmodem::{Progress, Xmodem};

use std::io;
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

use structopt::StructOpt;
use serial::core::{CharSize, BaudRate, StopBits, FlowControl, SerialDevice};

use parsers::{parse_width, parse_stop_bits, parse_flow_control, parse_baud_rate};

#[derive(StructOpt, Debug)]
#[structopt(about = "Write to TTY using the XMODEM protocol by default.")]
struct Opt {
    #[structopt(short = "i", help = "Input file (defaults to stdin if not set)", parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(short = "b", long = "baud", parse(try_from_str = "parse_baud_rate"),
                help = "Set baud rate", default_value = "115200")]
    baud_rate: BaudRate,

    #[structopt(short = "t", long = "timeout", parse(try_from_str),
                help = "Set timeout in seconds", default_value = "10")]
    timeout: u64,

    #[structopt(short = "w", long = "width", parse(try_from_str = "parse_width"),
                help = "Set data character width in bits", default_value = "8")]
    char_width: CharSize,

    #[structopt(help = "Path to TTY device", parse(from_os_str))]
    tty_path: PathBuf,

    #[structopt(short = "f", long = "flow-control", parse(try_from_str = "parse_flow_control"),
                help = "Enable flow control ('hardware' or 'software')", default_value = "none")]
    flow_control: FlowControl,

    #[structopt(short = "s", long = "stop-bits", parse(try_from_str = "parse_stop_bits"),
                help = "Set number of stop bits", default_value = "1")]
    stop_bits: StopBits,

    #[structopt(short = "r", long = "raw", help = "Disable XMODEM")]
    raw: bool,
}

fn main() {
    use serial::core::SerialPortSettings;

    let opt = Opt::from_args();
    let mut port = serial::open(&opt.tty_path).expect("path points to invalid TTY");

    // FIXME: Implement the `ttywrite` utility.
    let mut setting = port.read_settings().expect("failed to read initial ttysettings");
    setting.set_baud_rate(opt.baud_rate).expect("failed to set baud rate");
    setting.set_char_size(opt.char_width);
    setting.set_stop_bits(opt.stop_bits);
    setting.set_flow_control(opt.flow_control);
    port.write_settings(&setting).expect("failed to write new tty settings");;
    port.set_timeout(Duration::new(opt.timeout, 0)).expect("failed to set new timeout");

    if opt.raw {
        loop {
            if let Ok(len) = write_without_xmodem(&opt.input, &mut port) {
                println!("wrote {} bytes to input", len);
                break;
            }
        }
    } else {
        loop {
            let res = write_with_xmodem(&opt.input, &mut port);
            match res {
                Ok(len) => { 
                    println!("wrote {} bytes to input", len);
                    break;
                },
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            }
        }
    }

}

fn write_without_xmodem(input: &Option<PathBuf>, port: &mut serial::unix::TTYPort) -> io::Result<u64> {
    match input {
        Some(file_path) => io::copy(
            &mut File::open(file_path).expect("failed to open input file"),
            port
        ),
        None => io::copy(&mut io::stdin(), port),
    }
}

fn write_with_xmodem(input: &Option<PathBuf>, port: &mut serial::unix::TTYPort) -> io::Result<usize> {
    match input {
        Some(file_path) => Xmodem::transmit_with_progress(
            &File::open(file_path).expect("failed to open input file"),
            port,
            progress_fn
        ),
        None => Xmodem::transmit_with_progress(io::stdin(), port, progress_fn),
    }
}

fn progress_fn(p: Progress) {
    println!("Progress: {:?}", p);
}
