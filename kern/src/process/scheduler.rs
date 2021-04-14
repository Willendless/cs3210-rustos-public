use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use alloc::string::String;
use core::fmt;

use aarch64::*;
use kernel_api::{OsError, OsResult};

use crate::mutex::Mutex;
use crate::param::{PAGE_MASK, PAGE_SIZE, TICK, USER_IMG_BASE};
use crate::process::{Id, Process, State, Context};
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
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) {
        self.critical(|scheduler| scheduler.schedule_out(new_state, tf));
    }

    /// loop for scheduler kernel thread
    /// This function should be called after the initialization
    /// of the first use process, so that the system can bootstrap
    /// process abstraction
    pub fn switch_to(&self) -> ! {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to());

            if let Some(prev_id) = rtn {
                // prev process now not execute
                // maybe do some bookkeeping here
                // ex: clean dead process mem
                // if process(id is prev_id) => clean its resources
                continue;
            } else {
                // since currently we don't support nested interrupt
                // when there is no ready process, we will halt the cpu
                // aarch64::wfe();
            }
        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> ! {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    pub fn running_process_name(&self) -> String {
        self.critical(|scheduler| scheduler.running_thread_name())
    }

    pub fn running_process_tf(&self) -> usize {
        self.critical(|scheduler| {
            &(*scheduler.processes[scheduler.running_thread()].trap_frame) as *const TrapFrame as usize
        })
    }

    // TODO: refactor it
    pub fn running_process_sp(&self) -> u64 {
        self.critical(|scheduler| {
            scheduler.processes[scheduler.running_thread()].stack.top().as_u64()
        })
    }

    // TODO: refoctor it
    pub fn running_process_tf_debug(&self) -> TrapFrame {
        self.critical(|scheduler| {
            *scheduler.processes[scheduler.running_thread()].trap_frame
        })
    }

    // TODO: refactor it to check validitiy of buf
    pub fn getcwd(&self, buf: u64, size: usize) {
        self.critical(|scheduler| {
            let i = scheduler.running_thread();
            let p = &scheduler.processes[i];
            let wd = p.cwd.to_str().unwrap();
            p.write_vbuf(wd, buf.into(), wd.len().min(size));
        })
    }

    pub fn load<P: AsRef<shim::path::Path>>(&self, pn: P) {
        self.critical(|scheduler| {
            self.add(Process::load(pn).expect("load failed"));
        });
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {
        // enable timer interrupt
        Controller::new().enable(Interrupt::Timer1);
        // set timer TICK match
        timer::tick_in(TICK * 3);
        // register trap handler function
        crate::IRQ.register(Interrupt::Timer1, Box::new(move |tf: &mut TrapFrame| {
            timer::tick_in(TICK * 3);
            crate::SCHEDULER.switch(State::Ready, tf);
        }));

        self.init_user_process()
    }

    /// Set up first user process
    fn init_user_process(&self) -> !{
        // for _ in 0..4 {
        //     self.add(Process::load("/fib").expect("succeed creating process"));
        // }
        for _ in 0..1 {
            self.add(Process::load("/shell").expect("succeed creating process"));
        }
        self.switch_to()
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        *self.0.lock() = Some(Scheduler::new());
        kprintln!("scheduler:: initialize");
    }

    pub fn fork(&self, tf: &TrapFrame) -> OsResult<Id> {
        self.critical(|scheduler| scheduler.fork(tf))
    }

    pub fn get_next_tick_time(&self) -> core::time::Duration {
        self.critical(|scheduler| scheduler.processes[scheduler.running_thread()].next_tick_time.unwrap())
    }
}

#[derive(Debug)]
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
    context: Box<Context>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        Scheduler {
            processes: VecDeque::new(),
            last_id: None,
            context: Box::new(Default::default()),
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
        let new_id: u64;
        // set process id
        if let Some(id) = self.last_id {
            if let Some(res) = id.checked_add(1) {
                self.last_id = Some(res);
                process.trap_frame.tpidr_els = res;
                process.pid = res;
            } else {
                // process id overflow, release it?
                panic!("process id overflow");
            }
        } else {
            process.trap_frame.tpidr_els = 0;
            process.pid = 0;
            self.last_id = Some(0);
        }
        // kprintln!("add process {}", process.pid);
        // set process state
        process.state = State::Ready;
        new_id = process.pid;
        self.processes.push_back(process);
        Some(new_id)
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) {
        let thread_context_ptr: u64;
        let index = self.running_thread();
        let mut cur_thread = &mut self.processes[index];

        // TODO(store trap frame): consider remove redundant trap frame
        *cur_thread.trap_frame = *tf;

        cur_thread.state = new_state;
        thread_context_ptr = &(*cur_thread.context) as *const Context as u64;

        match cur_thread.state {
            State::Ready | State::Waiting(_) => {
                let running_process = self.processes.remove(index).unwrap();
                // kprintln!("prev: {:#?}", running_process.context);
                // kprintln!("process {} schedule out", running_process.pid);
                self.processes.push_back(running_process);
            },
            State::Dead => {
                // reclaim id
                let id = cur_thread.pid;
                if self.last_id.unwrap() == id {
                    self.last_id = id.checked_sub(1);
                }
                kprintln!("thread {} dead", id);
                // core::mem::drop(cur_thread);
                // remove from process queue
                // self.processes.remove(self.running_thread()).unwrap();
                // kprintln!("remove ok");
            }
            State::Start | State::Running => unreachable!(),
        }

        unsafe {
            asm!("mov x0, $0
                mov x1, $1
                bl switch_threads"
                ::"r"(thread_context_ptr), "r"(&(*self.context))
                :"x0", "x1", "x2"
                : "volatile");
        }

        // Waiting and Ready state thread may return back here
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self) -> Option<Id> {
        let mut i = 0;
        while i < self.processes.len() {
            let p = self.processes.get_mut(i).unwrap();
            if p.is_ready() {
                let thread_context_ptr: u64;
                let mut next_process = self.processes.remove(i).unwrap();
                let pid = next_process.pid;
                // set execution state
                next_process.state = State::Running;
                // set next tick time, for kernel state yield
                next_process.next_tick_time = Some(timer::next_tick_time(TICK * 3));
                // reset timer
                timer::tick_in(TICK * 3);

                let thread_context = &(*next_process.context) as *const Context as u64;
                // kprintln!("{:#?}", next_process);
                // push into queue
                // kprintln!("next:{:#?}", next_process.context);
                self.processes.push_front(next_process);

                // kprintln!("swtch to {} process", pid);
                // switch from scheduler to kernel thread
                unsafe {
                    asm!("mov x0, $0
                        mov x1, $1
                        bl switch_threads"
                        :: "r"(&(*self.context)), "r"(thread_context)
                        : "x0", "x1", "x2"
                        : "volatile");
                }
                
                return Some(pid);
            } else if p.is_dead() {
                self.processes.remove(i).unwrap();
            } else {
                i += 1;
            }
        }
        None
    }

    fn running_thread_name(&self) -> String {
        self.processes[self.running_thread()].name.clone()
    }

    /// TODO: This func may not work when change to multiprocessor arch
    fn running_thread(&self) -> usize {
        for (i, p) in self.processes.iter().enumerate() {
            match p.state {
                State::Running => return i,
                _ => continue,
            }
        }
        unreachable!()
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. The dead process's resource will be recycled by scheduler thread.
    fn kill(&mut self, tf: &mut TrapFrame) -> ! {
        // schedule out the current running process
        self.schedule_out(State::Dead, tf);
        unreachable!()
    }

    /// Fork current running process and add the new process into queue.
    fn fork(&mut self, tf: &TrapFrame) -> OsResult<Id> {
        let running_thread = self.running_thread();
        let mut fork_process = self.processes[running_thread].fork()?;
        // set child process's return value as 0
        *fork_process.trap_frame = *tf;
        fork_process.trap_frame.ttbr1_el1 = fork_process.vmap.as_ref().unwrap().get_baddr().as_u64();
        fork_process.trap_frame.tpidr_els = fork_process.pid;
        fork_process.trap_frame.x[0] = 0;
        fork_process.trap_frame.x[7] = 1;
        if let Some(id) = self.add(fork_process) {
            kprintln!("fork success, child's id: {}", id);
            Ok(id)
        } else {
            Err(OsError::IdOverflow)
        }
    }
}
