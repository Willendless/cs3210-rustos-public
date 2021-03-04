use crate::atags::raw;

pub use crate::atags::raw::{Core, Mem};
use core::{slice, str};

/// An ATAG.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Atag {
    Core(raw::Core),
    Mem(raw::Mem),
    Cmd(&'static str),
    Unknown(u32),
    None,
}

impl Atag {
    /// Returns `Some` if this is a `Core` ATAG. Otherwise returns `None`.
    pub fn core(self) -> Option<Core> {
        match self {
            Atag::Core(rawcore) => Some(rawcore),
            _ => None,
        }
    }

    /// Returns `Some` if this is a `Mem` ATAG. Otherwise returns `None`.
    pub fn mem(self) -> Option<Mem> {
        match self {
            Atag::Mem(rawmem) => Some(rawmem),
            _ => None,
        }
    }

    /// Returns `Some` with the command line string if this is a `Cmd` ATAG.
    /// Otherwise returns `None`.
    pub fn cmd(self) -> Option<&'static str> {
        match self {
            Atag::Cmd(rawcmd) => Some(rawcmd),
            _ => None,
        }
    }
}

// FIXME: Implement `From<&raw::Atag> for `Atag`.
impl From<&'static raw::Atag> for Atag {
    fn from(atag: &'static raw::Atag) -> Atag {
        // FIXME: Complete the implementation below.

        unsafe {
            match (atag.tag, &atag.kind) {
                (raw::Atag::CORE, &raw::Kind { core }) => Atag::Core(core),
                (raw::Atag::MEM, &raw::Kind { mem }) => Atag::Mem(mem),
                (raw::Atag::CMDLINE, &raw::Kind { ref cmd }) => {
                    let mut offset = 0;
                    let firstptr = &(cmd.cmd) as *const u8;
                    loop {
                        let addr = firstptr.add(offset);
                        if *addr == b'\0' {
                            break;
                        }
                        offset += 1;
                    }
                    let cmd = slice::from_raw_parts(firstptr, offset);
                    let cmd = str::from_utf8(cmd).expect("failed to convert to str");
                    Atag::Cmd(cmd)
                },
                (raw::Atag::NONE, _) => Atag::None,
                (id, _) => Atag::Unknown(id),
            }
        }
    }
}
