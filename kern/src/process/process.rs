use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use shim::io;
use shim::path::{Path, PathBuf};
use shim::const_assert_size;

use aarch64;
use smoltcp::socket::SocketHandle;

use crate::{VMM, FILESYSTEM, param::*};
use crate::process::{Stack, State, Context};
use crate::traps::TrapFrame;
use crate::vm::*;
use kernel_api::{OsError, OsResult};

use fat32::traits::FileSystem;
use fat32::traits::File;
use crate::fs::PiVFatHandle;

/// Type alias for the type of a process ID.
pub type Id = u64;

/// A structure that represents the complete state of a process.
#[derive(Debug)]
pub struct Process {
    /// Unique process id.
    pub pid: Id,
    /// The name of the process.
    pub name: String,
    /// TODO: The saved trap frame of a process.
    pub trap_frame: Box<TrapFrame>,
    /// The saved kernel thread stack.
    pub context: Box<Context>,
    /// The memory allocation used for the process's stack.
    pub stack: Stack,
    /// The page table describing the Virtual Memory of the process.
    pub vmap: Option<Box<UserPageTable>>,
    /// The open file table of the process.
    pub open_file_table: [Option<fat32::vfat::Entry<PiVFatHandle>>; 16],
    /// The current working directory of the process.
    pub cwd: PathBuf,
    /// The scheduling state of the process.
    pub state: State,
    /// The next tick time of the process.
    pub next_tick_time: Option<core::time::Duration>,
    // Lab 5 2.C
    // Socket handles held by the current process
    // pub sockets: Vec<SocketHandle>,
}

impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new(name: &str, kernel_thread: bool) -> OsResult<Process> {
        if let Some(stack) = Stack::new() {
            let mut context: Box<Context> = Box::new(Default::default());
            match kernel_thread {
                true => {
                    // test purpose
                    context.lr = kernel_thread_init as *const() as u64;
                    context.sp_el1 = stack.top().as_u64();
                },
                false => {
                    context.lr = fork_ret as *const() as u64;
                    context.sp_el1 = stack.top().as_u64();
                }
            }
            Ok(Process {
                pid: 0,
                name: name.to_string(),
                stack,
                context: context,
                trap_frame: Box::new(Default::default()),
                state: State::Start,
                vmap: match kernel_thread {
                    false => Some(Box::new(UserPageTable::new())),
                    true => None,
                },
                cwd: PathBuf::from("/"),
                open_file_table: Default::default(),
                next_tick_time: None
            })
        } else {
            Err(OsError::NoMemory)
        }
    }

    /// Loads a program stored in the given path by calling `do_load()` method.
    /// Sets trapframe `context` corresponding to its page table.
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
        info!("process: user program load succeed");
        p.trap_frame.sp_els = Self::get_stack_top().as_u64();
        p.trap_frame.elr_elx = Self::get_image_base().as_u64();
        p.trap_frame.ttbr0_el1 = VMM.get_baddr().as_u64();
        p.trap_frame.ttbr1_el1 = p.vmap.as_ref().unwrap().get_baddr().as_u64();
        p.trap_frame.spsr_elx = 0b11_0110_0000;
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
            let page = process.vmap.as_mut().expect("user process should have vmap").alloc(code_vaddr, PagePerm::RWX);
            let read_size = f.read(page)?;
            code_vaddr += read_size.into();
        }

        // assign heap memory
        code_vaddr = crate::allocator::util::align_up(code_vaddr.as_usize(), PAGE_SIZE).into();
        process.vmap.as_mut().expect("user process should have vmap").alloc(code_vaddr, PagePerm::RWX);

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
            State::Ready | State::Running => return true,
            State::Start => panic!("thread just started should not reach here"),
            State::Waiting(_) => {}
            State::Dead => return false,
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

    pub fn is_dead(&self) -> bool {
        match self.state {
            State::Dead => true,
            _ => false,
        }
    }

    /// Create a new process, copying the parent.
    pub fn fork(&mut self) -> OsResult<Process> {
        let mut p = Process::new("", false)?;
        p.cwd = self.cwd.clone();
        p.vmap.as_mut().unwrap().from(self.vmap.as_ref().unwrap());
        Ok(p)
    }

    /// Write data to buf begin from vaddr.
    pub fn write_vbuf(&self, data: &str, vaddr: VirtualAddr, size: usize) {
        let mut paddr = self.vmap.as_ref().unwrap().get_kaddr(vaddr);
        unsafe { core::ptr::copy(data.as_ptr(), paddr.as_mut_ptr(), size); }
    }
}

#[no_mangle]
extern "C" fn kernel_thread_init() {
    use crate::shell;
    loop {
        // unsafe { kprintln!("process_exe: EL{}", current_el()); } cannot call in el0
        shell::shell("process0> ");
    }
    // TODO: maybe return to user space
}

// A fork child's very first scheduling
// will switch to user process.
#[no_mangle]
extern "C" fn fork_ret() {
        // first use trap frame to restore context
        use crate::console::kprintln;

        // kprintln!("fork ret: {:#?}", SCHEDULER.running_process_tf_debug());
        use crate::SCHEDULER;
        unsafe {
            asm!("mov x28, $0
                mov sp, $1
                bl context_restore
                mov x0, x28
                ldp     x28, x29, [sp], #16
                ldp     lr, xzr, [sp], #16
                mov sp, x0
                mov x0, xzr
                eret"
                :: "r"(SCHEDULER.running_process_sp()), "r"(SCHEDULER.running_process_tf())
                :: "volatile");
        }
}
