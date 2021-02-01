use super::{Task, TaskId}; 
use alloc::task::Wake;
use alloc::{collections::BTreeMap, sync::Arc};
use core::task::Waker;
use core::task::{Context, Poll};
use crossbeam_queue::ArrayQueue;
use crate::serial_println;

/// # Executor
/// 
/// A much more optimized, and generally better executor than SimpleExecutor.
/// 
/// Stores tasks in a BTreeMap, where it holds the taskId and the Task.
/// 
/// Stores the queue as an `Arc<ArrayQueue<TaskId>>` so it can be used by the waker and executor.
/// the waker will push the woken ID to this queue, where the executor will then run the task
/// 
/// Waker cache stores the taskId and it's relevant waker
pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    /// Initialize a new Executor
    pub fn new() -> Self {
        serial_println!("Initialized task executor");
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    /// Spawn a new task. Will panic if the task already exists on the task map.
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("queue full");
    }
}

impl Executor {
    /// Iterate through our task_queue, to check what tasks are ready to run. Then run them
    fn run_ready_tasks(&mut self) {
        // destructure `self` to avoid borrow checker errors (will be fixed soon)
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Ok(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone())); // Instead of recreating a new waker every time, we use the waker already stored in the cache for this task
            let mut context = Context::from_waker(waker); // create a new context from the waker
            match task.poll(&mut context) { // check the task is ready
                Poll::Ready(()) => {
                    // task done -> remove it and its cached waker
                    tasks.remove(&task_id); // the task is done, we can remove it
                    waker_cache.remove(&task_id); // the waker is also no longer needed
                }
                Poll::Pending => {}
            }
        }
    }

    /// This function will run our executor. It is a diverging function, so will never return
    /// It will run in the background from our OS.
    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks(); // Run tasks indefinitely.
            self.sleep_if_idle(); // sleep if idle :P
        }
    }

    /// If we have no tasks, we should hlt to avoid wasting precious CPU time.
    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_interrupts_and_hlt};

        interrupts::disable(); // We should disable interrupts before checking the task queue, as between checking the task queue and sleeping,
                               // another interrupt could fire
        if self.task_queue.is_empty() {
            enable_interrupts_and_hlt(); // We re-enable interrupts and halt
        } else {
            interrupts::enable(); // we have tasks to run, just re-enable interrupts and don't halt
        }
    }
}

/// # TaskWaker
/// 
/// This struct stores the waker's ID, as well as a reference to the task_queue
/// 
/// When the task is ready to be run, we add the ID to the queue, where it will be run
struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    /// Create a new task, inputting the task's ID and a reference to the queue. We return a waker from this TaskWaker
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    /// Submit the task_id to the task queue (panic if it is full)
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

// This allows our `TaskWaker` to be used as a Waker (through the Arc struct)
impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}