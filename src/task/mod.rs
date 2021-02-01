pub mod simple_executor; // very basic, barebones executor (Executors manage the current tasks running)
pub mod executor; // Much better executor

use core::{future::Future, pin::Pin}; // Get the pin and futures we need to use async - pin works by making sure the position of the future
                                      // on the heap doesn't move, but instead stays (Which is important when multitasking!). It 'pins' it :D
use alloc::boxed::Box; // Boxes (so we can store it on the heap, as future doesn't have a known compile size)
use core::task::{Context, Poll}; // Allows us to poll the future
use core::sync::atomic::{AtomicU64, Ordering};

/// Each task must have its own unique ID, so we can specify what task is being woken
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    /// Create a new TaskId, by incrementing a static atomicU64 (so we get a new unique ID no matter what)
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}


/// # Task
/// 
/// This struct allows you to create a new asynchrynous task. It stores a `future`
pub struct Task {
    id: TaskId, // Our tasks current task ID
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Create a new task
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }
}


impl Task {
    /// Poll the task, to check if has finished. Return the poll, so the user calling it can check (ie, the result
    /// is critical to the next stage of the program).
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context) // the poll method requires a mutable future, so we borrow the pin as a mutable ref
    }
}