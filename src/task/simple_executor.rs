use super::Task; // Import our tasks
use alloc::collections::VecDeque; // VecDeque (allows us to insert tasks on either side of the vec, so we can prioritise certain tasks)
use crate::println;

/// # SimpleExecutor
/// 
/// SimpleExecutor is a simple executor of async tasks, and shouldn't be used in production
/// 
/// It implements a very basic `VecDeque` system for storing tasks, and has methods for spawning and waking tasks.
pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    /// Create a new SimpleExecutor struct
    pub fn new() -> SimpleExecutor {
        println!("[LOG] SimpleExecutor initialized");

        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    /// spawn a new task on the task queue (note: this consumes the task)
    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }
}

use core::task::{Context, Poll}; /// Context and poll for our run

impl SimpleExecutor {
    /// Run the tasks. We iterate through every task, check it is finished.
    /// 
    /// If it is, we remove it from the task array. Otherwise we keep it in. We loop forever for every task in the queue
    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker(); // Create a waker (wakers notify our executor the task has finished, and wake the task.)
            let mut context = Context::from_waker(&waker); // Create a context around our waker
            match task.poll(&mut context) { // we check the task has finished with our waker
                Poll::Ready(()) => {} // task done
                Poll::Pending => self.task_queue.push_back(task), // task not yet done
            }
        }
    }
}

use core::task::{Waker, RawWaker}; // Waker struct and RawWaker trait which implements waker functions
/// RawWaker requires a VTable, which is used in programming to support dynamic dispatch.
/// This allows RawWaker to know what functions to call when it is Cloned, Woken or Dropped. As it is run at
/// runtime, it can't know, hence why we use a VTable to define it for us. This is also why it is unsafe to create
/// a waker from raw - it cannot verify the RawWaker has the required functions.


/// Unsafe as we need to ensure that the Waker has the required requirements.
/// 
/// This function creates a waker from a RawWaker
fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}


/// VTable implemention for RawPointer
use core::task::RawWakerVTable;

/// Define the creation of a rawpointer
fn dummy_raw_waker() -> RawWaker {
    /// What to do when not doing anything
    fn no_op(_: *const ()) {}
    /// What to do when cloned
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }
    /// We use the above functions to define what should be done on clone, drop and wake for our
    /// dummy rawpointer in the vtable. Here, we clone, then do nothing for the rest
    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);

    /// We then pass a pointer to data on the heap to pass to the waker. as we aren't using any data,
    /// we just point to null. We then include the table to define the functions needed at runtime.
    RawWaker::new(0 as *const (), vtable)
}