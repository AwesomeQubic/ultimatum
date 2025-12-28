use std::{ffi::c_uint, mem::zeroed, net::SocketAddr, num::NonZero, sync::LazyLock, thread::available_parallelism};

use liburing_rs::*;

use crate::{settings::{self, get_settings}, tasks};

pub const IO_URING_SIZE: c_uint = 512;

pub fn burn() {}

pub fn worker() {
    let settings = get_settings();

    let burn = timespec::from(settings.burn_time);

    let mut io = unsafe { zeroed::<io_uring>() };

    unsafe {
        //SAFETY: Se
        io_uring_queue_init(
            IO_URING_SIZE,
            &raw mut io,
            IORING_SETUP_SINGLE_ISSUER | IORING_SETUP_DEFER_TASKRUN,
        );
    }

    let tasking = settings.connections.div_ceil(settings.threads.get());

    let mut tasking = tasks::ThreadLocalTasking::setup(&raw mut io, tasking, &settings);

    loop {

    }
}
