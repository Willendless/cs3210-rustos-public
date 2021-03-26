use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use shim::io;
use shim::path::Path;

use aarch64;

use crate::{VMM, FILESYSTEM, param::*};
use crate::process::{Stack, State};
use crate::traps::TrapFrame;
use crate::vm::*;
use kernel_api::{OsError, OsResult};

use fat32::traits::FileSystem;
use fat32::traits::File;

/// Type alias for the type of a process ID.
pub type Id = u64;

/// A structure that represents the complete state of a process.
#[derive(Debug)]
pub struct Process {
    /// The name of the process.
    pub name: String,
    /// The saved trap frame of a process.
    pub context: Box<TrapFrame>,
    /// The memory allocation used for the process's stack.
    pub stack: Stack,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Option<Box<UserPageTable>>,
    /// The scheduling state of the process.
    pub state: State,
}

impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new(name: &str, kernel_thread: bool) -> OsResult<Process> {
        if let Some(stack) = Stack::new() {
            Ok(Process {
                name: name.to_string(),
                stack,
                context: Box::new(Default::default()),
                state: State::Ready,
                vmap: match kernel_thread {
                    false => Some(Box::new(UserPageTable::new())),
                    true => None,
                },
            })
        } else {
            Err(OsError::NoMemory)
        }
    }

    /// Load a program stored in the given path by calling `do_load()` method.
    /// Set trapframe `context` corresponding to the its page table.
    /// `sp` - the address of stack top
    /// `elr` - the address of image base.
    /// `ttbr0` - the base address of kernel page table
    /// `ttbr1` - the base address of user page table
    /// `spsr` - `F`, `A`, `D` bit should be set.
    ///
    /// Returns Os Error if do_load fails.
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use crate::VMM;
        use crate::console::kprintln;

        let mut p = Process::do_load(pn)?;
        p.context.sp_els = Self::get_stack_top().as_u64();
        p.context.elr_elx = Self::get_image_base().as_u64();
        p.context.ttbr0_el1 = VMM.get_baddr().as_u64();
        p.context.ttbr1_el1 = p.vmap.as_ref().unwrap().get_baddr().as_u64();
        p.context.spsr_elx = 0b11_0110_0000;
        Ok(p)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        // use crate::console::kprintln;
        let mut f = FILESYSTEM.open_file(pn.as_ref().clone())?;
        let mut process = Self::new(pn.as_ref().clone().to_str().unwrap(), false)?;
        // assign memory page for code
        let mut code_vaddr = Self::get_image_base();
        while !f.is_end() {
            use io::Read;
            let page = process.vmap.as_mut().expect("use process should have vmap").alloc(code_vaddr, PagePerm::RWX);
            let read_size = f.read(page)?;
            code_vaddr += read_size.into();
        }
        // stack segment
        let stack_vaddr = Self::get_stack_base();
        process.vmap.as_mut().unwrap().alloc(stack_vaddr, PagePerm::RW);
        Ok(process)
    }

    /// Returns the highest `VirtualAddr` that is supported by this system.
    pub fn get_max_va() -> VirtualAddr {
        VirtualAddr::from(USER_MAX_VM_SIZE + USER_IMG_BASE)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// memory space.
    pub fn get_image_base() -> VirtualAddr {
        VirtualAddr::from(USER_IMG_BASE)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// process's stack.
    pub fn get_stack_base() -> VirtualAddr {
        VirtualAddr::from(USER_STACK_BASE)
    }

    /// Returns the `VirtualAddr` represents the top of the user process's
    /// stack.
    pub fn get_stack_top() -> VirtualAddr {
        VirtualAddr::from(core::usize::MAX & !(16 - 1))
    }

    /// Returns `true` if this process is ready to be scheduled.
    ///
    /// This functions returns `true` only if one of the following holds:
    ///
    ///   * The state is currently `Ready`.
    ///
    ///   * An event being waited for has arrived.
    ///
    ///     If the process is currently waiting, the corresponding event
    ///     function is polled to determine if the event being waiting for has
    ///     occured. If it has, the state is switched to `Ready` and this
    ///     function returns `true`.
    ///
    /// Returns `false` in all other cases.
    pub fn is_ready(&mut self) -> bool {
        match self.state {
            State::Ready => return true,
            State::Running => return false,
            State::Dead => return false,
            State::Waiting(_) => {}
        }
        // handle waiting state process
        let mut state = core::mem::replace(&mut self.state, State::Ready);
        if let State::Waiting(ref mut event) = state {
            if event(self) {
                true
            } else {
                self.state = state;
                false
            }
        } else {
            unreachable!();
        }
    }
}
