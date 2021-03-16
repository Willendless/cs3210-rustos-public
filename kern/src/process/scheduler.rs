use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use core::fmt;

use aarch64::*;

use crate::mutex::Mutex;
use crate::param::{PAGE_MASK, PAGE_SIZE, TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State};
use crate::traps::TrapFrame;
use crate::traps::irq::IrqHandler;
use crate::VMM;
use pi::interrupt::{Controller, Interrupt};
use pi::timer;

use crate::console::{kprintln, kprint};

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
    pub fn critical<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Scheduler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("scheduler uninitialized"))
    }


    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
        self.critical(move |scheduler| scheduler.add(process))
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| scheduler.schedule_out(new_state, tf));
        self.switch_to(tf)
    }

    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            if let Some(id) = rtn {
                return id;
            }
            aarch64::wfe();
        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {
        if let Ok(mut first_process) = Process::new() {
            // enable timer interrupt
            Controller::new().enable(Interrupt::Timer1);
            // set timer TICK match
            timer::tick_in(TICK * 3);
            // register trap handler function
            crate::IRQ.register(Interrupt::Timer1, Box::new(move |tf: &mut TrapFrame| {
                timer::tick_in(TICK * 3);
                crate::SCHEDULER.switch(State::Ready, tf);
            }));

            // faking trap frame
            let mut tf: TrapFrame = Default::default();
            self.switch_to(&mut tf);

            // first use trap frame restore context
            // if not reset x28,29,30, exactly 6 instructions
            // then reset sp
            // and clear x0
            unsafe {
                asm!("mov sp, $0
                    bl context_restore
                    ldp     x28, x29, [sp], #16
                    ldp     lr, xzr, [sp], #16
                    ldr x0, =_start
                    mov sp, x0
                    mov x0, xzr
                    eret":: "r"(&tf):: "volatile");
            }
        }
        panic!("failed to allocate memory for process")
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        *self.0.lock() = Some(Scheduler::new());

        // init processes
        let init_func = [process_exe_0, process_exe_1, process_exe_2];
        for func in init_func.into_iter() {
            kprintln!("process *");
            let mut process = Process::new().unwrap();
            // set trap frame
            process.context.elr_elx = *func as *const() as u64;
            // from el2 to el1 we use #0x3c5, here we use #0x360
            // [9:8]: DA
            // [7:6]: IF unmask irq
            // 0101: EL1h, 0: EL0t
            process.context.spsr_elx = 0b11_0110_0000;
            // set el0 to top of stack
            process.context.sp_els = process.stack.top().as_u64();
            self.add(process);
        }
    }

    // The following method may be useful for testing Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    // pub fn test_phase_3(&self, proc: &mut Process){
    //     use crate::vm::{VirtualAddr, PagePerm};
    //
    //     let mut page = proc.vmap.alloc(
    //         VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);
    //
    //     let text = unsafe {
    //         core::slice::from_raw_parts(test_user_process as *const u8, 24)
    //     };
    //
    //     page[0..24].copy_from_slice(text);
    // }
}

#[no_mangle]
extern "C" fn process_exe_0() {
    use crate::shell;
    loop {
        // unsafe { kprintln!("process_exe: EL{}", current_el()); } cannot call in el0
        shell::shell("process0> ");
    }
}

#[no_mangle]
extern "C" fn process_exe_1() {
    use crate::shell;
    loop {
        // unsafe { kprintln!("process_exe: EL{}", current_el()); } cannot call in el0
        shell::shell("process1> ");
    }
}

#[no_mangle]
extern "C" fn process_exe_2() {
    use crate::shell;
    loop {
        // unsafe { kprintln!("process_exe: EL{}", current_el()); } cannot call in el0
        shell::shell("process2> ");
    }
}

#[derive(Debug)]
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        Scheduler {
            processes: VecDeque::new(),
            last_id: None
        }
    }

    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add(&mut self, mut process: Process) -> Option<Id> {
        // set process id
        if let Some(id) = self.last_id {
            if let Some(res) = id.checked_add(1) {
                self.last_id = Some(res);
                process.context.tpidr_els = res;
            } else {
                // process id overflow, release it?
                panic!("process id overflow");
            }
        } else {
            process.context.tpidr_els = 0;
            self.last_id = Some(0);
        }
        // set process state
        process.state = State::Ready;
        self.processes.push_back(process);
        for  p in self.processes.iter_mut() {
            if p.is_ready() {
                return Some(p.context.tpidr_els as Id);
            }
        }
        None
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) -> bool {
        let current_id = tid_el0();
        for (i, p) in self.processes.iter_mut().enumerate() {
            if p.context.tpidr_els == current_id {
                let mut cur_process = self.processes.remove(i).unwrap();
                // store context
                *cur_process.context = *tf;
                // update process state
                // kprintln!("shedule_out: {}", current_id);
                cur_process.state = new_state;
                // push into queue
                self.processes.push_back(cur_process);
                return true;
            }
        }
        false
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        for (i, p) in self.processes.iter_mut().enumerate() {
            if p.is_ready() {
                let mut next_process = self.processes.remove(i).unwrap();
                let pid = next_process.context.tpidr_els;
                // restore context
                *tf = *next_process.context;
                // set execution state
                next_process.state = State::Running;
                // push into queue
                self.processes.push_front(next_process);
                // kprintln!("scheduler::switch_to: {}: Running", pid);
                return Some(pid);
            }
        }
        None
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. Removes the dead process from the queue, drop the
    /// dead process's instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        // schedule out the current running process
        if self.schedule_out(State::Dead, tf) {
            let dead_process = self.processes.pop_back().unwrap();
            let dead_id = dead_process.context.tpidr_els;
            // reclaim id
            if let Some(last_id) = self.last_id {
                if last_id == dead_id {
                    self.last_id = last_id.checked_sub(1);
                }
            }
            // drop process instance
            Some(dead_id)
        } else {
            None
        }
    }
}

pub extern "C" fn  test_user_process() -> ! {
    loop {
        let ms = 10000;
        let error: u64;
        let elapsed_ms: u64;

        unsafe {
            asm!("mov x0, $2
              svc 1
              mov $0, x0
              mov $1, x7"
                 : "=r"(elapsed_ms), "=r"(error)
                 : "r"(ms)
                 : "x0", "x7"
                 : "volatile");
        }
    }
}
