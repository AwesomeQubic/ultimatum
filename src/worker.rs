use std::{
    collections::VecDeque,
    thread::{self, JoinHandle},
};

use liburing_rs::*;

use crate::{
    settings::get_settings,
    stats::{self, Statistics},
    tasks,
    uring::ThreadIo,
};

pub fn burn() {
    let settings = get_settings();
    let join_handles: Vec<JoinHandle<Statistics>> = (0..settings.threads.get())
        .map(|_| thread::spawn(worker))
        .collect();
    let mut our_stats = Statistics::default();
    for ele in join_handles.into_iter() {
        our_stats.merge(ele.join().unwrap());
    }
    stats::print_stats_final(&our_stats);
}

pub fn worker() -> Statistics {
    let settings = get_settings();

    let mut stats = Statistics::default();
    let mut io = ThreadIo::create();
    let mut tasking = tasks::ThreadLocalTasking::setup(&mut io, &mut stats);

    let sqe = io.push().sqe();
    unsafe {
        io_uring_prep_timeout(sqe, &settings.burn_time as *const __kernel_timespec, 0, 0);
        io_uring_sqe_set_data64(sqe, u64::MAX);
    };

    //Found this to be a bit faster
    let mut out = VecDeque::new();
    loop {
        io.wait_for_more(&mut out);
        let mut last = false;
        while let Some(cqe) = out.pop_front() {
            if cqe.user_data == u64::MAX {
                last = true;
                continue;
            }

            tasking.progress(&mut io, cqe, &mut stats);
        }

        if last {
            break;
        }
    }

    drop(io);

    stats
}
