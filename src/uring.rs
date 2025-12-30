use std::{collections::VecDeque, marker::PhantomData, mem::zeroed, ptr, time::Instant};

use libc::c_uint;
use liburing_rs::*;

pub const IO_URING_SIZE: c_uint = 512;

pub struct ThreadIo {
    ring: io_uring,
    //Force !Sync and !Send on stable
    phantom: PhantomData<*const ()>,
}

impl ThreadIo {
    pub fn create() -> Self {
        let mut io_uring: io_uring = unsafe { zeroed() };
        let r = unsafe {
            io_uring_queue_init(
                IO_URING_SIZE,
                &raw mut io_uring,
                IORING_SETUP_SINGLE_ISSUER | IORING_SETUP_DEFER_TASKRUN,
            )
        };

        assert_eq!(r, 0);

        Self {
            ring: io_uring,
            phantom: PhantomData,
        }
    }

    /// Get space in ring for next SQE
    /// 
    /// # SAFETY
    /// 
    /// io_uring_sqe can not be written to once ThreadIo goes out of scope
    #[inline]
    pub unsafe fn push(&mut self) -> *mut io_uring_sqe {
        let mut sqe = unsafe { io_uring_get_sqe(&raw mut self.ring) };

        if sqe.is_null() {
            unsafe { io_uring_submit(&raw mut self.ring) };
            let sqe_retry = unsafe { io_uring_get_sqe(&raw mut self.ring) };

            if sqe_retry.is_null() {
                panic!("Something went wrong");
            }

            sqe = sqe_retry;
        }

        sqe
    }

    #[inline]
    pub fn wait_for_more(&mut self, out_buf: &mut VecDeque<io_uring_cqe>) -> Instant {
        unsafe {
            let out = io_uring_submit_and_wait(&raw mut self.ring, 1);
            assert!(out >= 0, "Error while submitting");

            let cycle_time = Instant::now();

            let mut i = 0;
            io_uring_for_each_cqe(&raw mut self.ring, |x| {
                out_buf.push_back(ptr::read(x));
                i += 1;
            });
            io_uring_cq_advance(&raw mut self.ring, i);

            cycle_time
        }
    }

    pub fn inner(&mut self) -> *mut io_uring {
        &raw mut self.ring
    }
}

impl Drop for ThreadIo {
    fn drop(&mut self) {
        unsafe {
            io_uring_queue_exit(&raw mut self.ring);
        }
    }
}
