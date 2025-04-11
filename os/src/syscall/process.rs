//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::mm::{PageTable, VirtAddr, VirtPageNum};
use crate::task::{__sys_delvm, __sys_getvm, current_user_token};
use crate::task::{
    __sys_trace, change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,
};
use crate::timer::get_time_us;
use core::mem::size_of;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let page_table = PageTable::from_token(current_user_token());
    let start = _ts as usize;
    let end: usize = start + size_of::<TimeVal>();

    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    let start_va = VirtAddr::from(start);
    let _end_va = VirtAddr::from(end);
    let vpn = start_va.floor();

    unsafe {
        if start >> 12 == (end - 1) >> 12 {
            match page_table.translate(vpn) {
                Some(page_table_entry) => {
                    let ppn = page_table_entry.ppn();
                    let phys_addr = (ppn.0 << 12) + (start % PAGE_SIZE);
                    *(phys_addr as *mut TimeVal) = time_val;
                }
                None => {
                    trace!("kernel: sys_get_time failed: page translation error");
                    return -1;
                }
            }
        } else {
            let first_page_end = (vpn.0 + 1) * PAGE_SIZE;
            let first_len = first_page_end - start;
            let second_len = end - first_page_end;

            match page_table.translate(vpn) {
                Some(page_table_entry) => {
                    let ppn1 = page_table_entry.ppn();
                    let phys_addr1 = (ppn1.0 << 12) + (start % PAGE_SIZE);

                    match page_table.translate(VirtPageNum(vpn.0 + 1)) {
                        Some(page_table_entry) => {
                            let ppn2 = page_table_entry.ppn();
                            let phys_addr2 = (ppn2.0 << 12) + (first_page_end % PAGE_SIZE);

                            core::ptr::copy_nonoverlapping(
                                &time_val as *const _ as *const u8,
                                phys_addr1 as *mut u8,
                                first_len,
                            );

                            core::ptr::copy_nonoverlapping(
                                (&time_val as *const _ as *const u8).add(first_len),
                                phys_addr2 as *mut u8,
                                second_len,
                            );
                        }
                        None => {
                            trace!("kernel: sys_get_time failed: second page translation error");
                            return -1;
                        }
                    }
                }
                None => {
                    trace!("kernel: sys_get_time failed: first page translation error");
                    return -1;
                }
            }
        }
    }

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        0 => {
            let id_va = VirtAddr::from(_id as usize);
            let vpn = id_va.floor();
            let page_table = PageTable::from_token(current_user_token());
            match page_table.translate(vpn) {
                Some(page_table_entry) => {
                    if page_table_entry.bits & 1 << 1 != 0
                        && page_table_entry.bits & 1 != 0
                        && page_table_entry.bits & 1 << 4 != 0
                    {
                        let ppn = page_table_entry.ppn();
                        let phys_addr = (ppn.0 << 12) + (_id % PAGE_SIZE);
                        unsafe {
                            return *(phys_addr as *mut usize) as isize;
                        }
                    } else {
                        trace!("kernel: sys_trace failed: insufficient permissions");
                        return -1;
                    }
                }
                None => {
                    trace!("kernel: sys_trace failed: address translation error");
                    return -1;
                }
            }
        }
        1 => {
            let id_va = VirtAddr::from(_id as usize);
            let vpn = id_va.floor();
            let page_table = PageTable::from_token(current_user_token());
            match page_table.translate(vpn) {
                Some(page_table_entry) => {
                    if page_table_entry.bits & 1 << 2 != 0
                        && page_table_entry.bits & 1 != 0
                        && page_table_entry.bits & 1 << 4 != 0
                    {
                        let ppn = page_table_entry.ppn();
                        let phys_addr = (ppn.0 << 12) + (_id % PAGE_SIZE);
                        unsafe {
                            *(phys_addr as *mut usize) = _data;
                        }
                        return 0;
                    } else {
                        trace!("kernel: sys_trace failed: insufficient permissions");
                        return -1;
                    }
                }
                None => {
                    trace!("kernel: sys_trace failed: address translation error");
                    return -1;
                }
            }
        }
        2 => __sys_trace(_id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap ");
    __sys_getvm(_start, _len, _port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap ");
    __sys_delvm(_start, _len)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
