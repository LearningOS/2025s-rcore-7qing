use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    let mutexid: isize;
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        mutexid = id as isize;
    } else {
        process_inner.mutex_list.push(mutex);
        mutexid = process_inner.mutex_list.len() as isize - 1;
    }
    process_inner.set_m_available(mutexid as usize, 1);
    trace!("mutexavailable_size: {}", mutexid as usize);
    for task_id in 0..process_inner.tasks.len() {
        let task = process_inner.get_task(task_id);
        let mut task_inner = task.inner_exclusive_access();
        trace!("mutexavailable_size: {}", mutexid as usize);
        task_inner.set_m_allocation((mutexid + 1) as usize, 0);
        task_inner.set_m_need((mutexid + 1) as usize, 0);
    }
    return mutexid;
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.set_m_need(mutex_id, 1);
    drop(task_inner);
    drop(task);
    if process_inner.dead_lock_dectect {
        let mut work = process_inner.m_available.clone();
        let task_len = process_inner.tasks.len();
        let mut finish = vec![false; task_len];

        loop {
            let mut found = false;
            for task_id in 0..task_len {
                if finish[task_id] {
                    continue;
                }
                let task = process_inner.get_task(task_id);
                let mut task_inner = task.inner_exclusive_access();
                let needs_adjustment = work.iter().enumerate().any(|(mutex_id, &mutex_remain)| {
                    task_inner.set_m_need(mutex_id, 0);
                    task_inner.m_need[mutex_id] > mutex_remain
                });
                if !needs_adjustment {
                    finish[task_id] = true;
                    work.iter_mut().enumerate().for_each(|(pos, ptr)| {
                        task_inner.set_m_allocation(pos, 0);
                        *ptr += task_inner.m_allocation[pos];
                    });
                    found = true;
                }
            }
            if !found {
                break;
            }
        }
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        if finish.iter().any(|x| *x == false) {
            task_inner.m_need[mutex_id] -= 1;
            return -0xDEAD;
        }
    }
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    task_inner.m_need[mutex_id] -= 1;
    task_inner.m_allocation[mutex_id] += 1;
    process_inner.m_available[mutex_id] -= 1;
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    drop(task_inner);
    drop(task);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    process_inner.m_available[mutex_id] += 1;
    task_inner.m_allocation[mutex_id] -= 1;

    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    drop(task_inner);
    drop(task);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    process_inner.set_s_available(id, res_count);
    trace!("available_sem_size: {}", process_inner.s_available.len(),);
    for task_id in 0..process_inner.tasks.len() {
        let task = process_inner.get_task(task_id);
        let mut task_inner = task.inner_exclusive_access();
        trace!("allocation_task_id[{}] semaphore_id[{}]", task_id, id);
        task_inner.set_s_allocation(id + 1, 0);
        task_inner.set_s_need(id + 1, 0);
    }
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    process_inner.s_available[sem_id] += 1;

    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    trace!(
        "tid:{} sem_size: {}",
        task_inner.res.as_ref().unwrap().tid,
        task_inner.s_allocation.len(),
    );
    task_inner.s_allocation[sem_id] -= 1;

    drop(process_inner);
    drop(task_inner);
    drop(task);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    task_inner.set_s_need(sem_id, 1);

    drop(task_inner);
    drop(task);
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    if process_inner.dead_lock_dectect {
        let mut work = process_inner.s_available.clone();
        let task_len = process_inner.tasks.len();
        let mut finish = vec![false; task_len];

        loop {
            let mut found = false;

            for task_id in 0..task_len {
                if finish[task_id] {
                    continue;
                }

                let task = process_inner.get_task(task_id);
                let mut task_inner = task.inner_exclusive_access();

                let can_proceed = !work.iter().enumerate().any(|(sem_id, &sem_remain)| {
                    task_inner.set_s_need(sem_id, 0);
                    task_inner.s_need[sem_id] > sem_remain
                });

                if can_proceed {
                    finish[task_id] = true;
                    work.iter_mut().enumerate().for_each(|(pos, ptr)| {
                        task_inner.set_s_allocation(pos, 0);
                        *ptr += task_inner.s_allocation[pos];
                    });
                    found = true;
                }
            }

            if !found {
                break;
            }
        }

        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        if finish.iter().any(|x| *x == false) {
            task_inner.s_need[sem_id] -= 1;
            return -0xDEAD;
        }
    }

    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    if process_inner.s_available[sem_id] > 0 {
        task_inner.s_need[sem_id] -= 1;
        task_inner.set_s_allocation(sem_id, 1);
        process_inner.s_available[sem_id] -= 1;
    }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    drop(task_inner);
    drop(task);
    sem.down();
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    match _enabled {
        1 => current_process().inner_exclusive_access().dead_lock_dectect = true,
        0 => current_process().inner_exclusive_access().dead_lock_dectect = false,
        _ => return -1,
    };
    0
}
