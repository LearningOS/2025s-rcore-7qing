//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
    big_stride: u32,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            big_stride: (1 << 32 - 1) / 2,
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let mut min_task = self.ready_queue.front().cloned().unwrap();

        let mut min_task_index = 0;
        for (index, task) in self.ready_queue.iter().enumerate() {
            let task_stride = task.inner_exclusive_access().stride; // 临时借用数据
            let min_task_stride = min_task.inner_exclusive_access().stride;
            if task_stride < min_task_stride {
                min_task_index = index;
                // 更新最小任务（clone 任务对象）
                min_task = task.clone();
            }
        }
        self.ready_queue.remove(min_task_index);
        Some(min_task)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

/// Add stride to the process
pub fn add_stride(task: &Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_stride");
    let mut task_inner = task.inner_exclusive_access();
    task_inner.stride += TASK_MANAGER.exclusive_access().big_stride / (task_inner.priority as u32);
    drop(task_inner);
}
