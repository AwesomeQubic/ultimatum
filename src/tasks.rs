use std::{
    net::SocketAddr,
    os::{fd::RawFd, raw::c_void},
    pin::Pin,
};

use libc::{in6_addr, in_addr, sockaddr_in, sockaddr_in6, AF_INET};
use liburing_rs::*;
use nix::errno::Errno;

use crate::{
    settings::{get_settings, Protocol},
    stats::Statistics,
    uring::ThreadIo,
};

pub struct ThreadLocalTasking {
    tasks: Box<[Task]>,
    memory: memmap2::MmapMut,
}

struct TaskBuf<'a> {
    send: &'a mut [u8],
    receive: &'a mut [u8],
}

const TASK_BUF: usize = 2 * 4096;

impl ThreadLocalTasking {
    pub fn setup(io: &mut ThreadIo, stats: &mut Statistics) -> ThreadLocalTasking {
        let settings = get_settings();
        let connections = settings.connections_per_thread();

        let len = connections * TASK_BUF;
        let mut mapped = memmap2::MmapMut::map_anon(len).expect("Could not map memory");

        unsafe {
            let io_vec = iovec {
                iov_base: mapped.as_mut_ptr() as *mut c_void,
                iov_len: len,
            };

            assert!(
                io_uring_register_buffers(io.inner(), &raw const io_vec, 1) >= 0,
                "Could not register buffer"
            );

            assert!(
                io_uring_register_files_sparse(io.inner(), connections as u32) == 0,
                "Could not alloc direct files"
            );

            let mut tasks = Vec::with_capacity(connections);
            for i in 0..connections {
                tasks.push(Task {
                    index: i,
                    fd: 0,
                    dumb_rand: i as u64,
                    state: TaskState::default(),
                    addr: None,
                    addr6: None,
                });
            }

            for ele in tasks.iter_mut() {
                let index = ele.index;
                let mut buf = buffers_for_task(&mut mapped, index);
                ele.progress(None, io, &mut buf, stats);
            }

            ThreadLocalTasking {
                tasks: tasks.into_boxed_slice(),
                memory: mapped,
            }
        }
    }

    pub fn progress(&mut self, io: &mut ThreadIo, cqe: io_uring_cqe, stats: &mut Statistics) {
        let index = cqe.user_data as usize;
        let mut buf = buffers_for_task(&mut self.memory, index);
        self.tasks[index].progress(Some(cqe), io, &mut buf, stats);
    }
}

fn buffers_for_task<'a>(map: &'a mut memmap2::MmapMut, index: usize) -> TaskBuf<'a> {
    let array = &mut map[(index * TASK_BUF)..(index * TASK_BUF + TASK_BUF)];
    let (send, receive) = array.split_at_mut(4096);
    TaskBuf { send, receive }
}

struct Task {
    index: usize,
    fd: RawFd,
    dumb_rand: u64,
    state: TaskState,

    //Adresses
    addr: Option<Pin<Box<sockaddr_in>>>,
    addr6: Option<Pin<Box<sockaddr_in6>>>,
}

#[derive(Default, Debug)]
pub enum TaskState {
    #[default]
    NewSock,
    Connect,
    Setup,
    Send,
    Receive,
}

impl Task {
    pub fn progress(
        &mut self,
        cqe: Option<io_uring_cqe>,
        ring: &mut ThreadIo,
        buf: &mut TaskBuf<'_>,
        stats: &mut Statistics,
    ) {
        unsafe {
            if let Some(cqe) = cqe.as_ref() {
                if cqe.flags & IORING_CQE_F_MORE != 0 {
                    return;
                }
            }

            let sqe = ring.push().sqe();
            io_uring_sqe_set_data64(sqe, self.index as u64);

            match self.state {
                TaskState::NewSock => {
                    let settings = get_settings();
                    let sock_type = match settings.proto {
                        Protocol::Tcp => SOCK_STREAM,
                        Protocol::Udp => SOCK_DGRAM,
                    };

                    let domain = match settings.target {
                        SocketAddr::V4(_) => AF_INET,
                        SocketAddr::V6(_) => AF_INET6 as i32,
                    };
                    io_uring_prep_socket(sqe, domain, sock_type as i32, 0, 0);
                    self.state = TaskState::Connect;
                }
                TaskState::Connect => {
                    let Some(cqe) = cqe else {
                        panic!("Invalid state")
                    };

                    if cqe.res < 0 {
                        panic!(
                            "Non recoverable error whilst creating a socket {:?}",
                            Errno::from_raw(-cqe.res)
                        );
                    }

                    self.fd = cqe.res;

                    self.make_connect(sqe);

                    self.state = TaskState::Setup;
                }
                TaskState::Setup => {
                    let Some(cqe) = cqe else {
                        panic!("Invalid state")
                    };

                    if cqe.res < 0 {
                        self.make_connect(sqe);
                        stats.increment_connect_fail();
                        return;
                    }

                    self.addr = None;
                    self.addr6 = None;

                    let out: &mut [u64] = bytemuck::cast_slice_mut(buf.send);
                    for (index, ele) in out.iter_mut().enumerate() {
                        self.dumb_rand = self.dumb_rand.wrapping_add(*ele);
                        *ele ^= self.dumb_rand + (index as u64);
                    }

                    io_uring_prep_send_zc(
                        sqe,
                        self.fd,
                        buf.send.as_ptr() as *const c_void,
                        buf.send.len(),
                        0,
                        0,
                    );

                    self.state = TaskState::Receive;
                }
                TaskState::Send => {
                    let Some(cqe) = cqe else {
                        panic!("Invalid state")
                    };

                    if cqe.res < 0 {
                        eprintln!("Error whilst Receiving {}", Errno::from_raw(-cqe.res));
                    }

                    if buf.receive != buf.send {
                        stats.increment_wrong_returns();
                    } else {
                        stats.increment_successful_returns();
                    }

                    let out: &mut [u64] = bytemuck::cast_slice_mut(buf.send);
                    for ele in out.iter_mut() {
                        self.dumb_rand = self.dumb_rand.wrapping_add(*ele);
                        *ele ^= self.dumb_rand;
                    }

                    io_uring_prep_send_zc(
                        sqe,
                        self.fd,
                        buf.send.as_ptr() as *const c_void,
                        buf.send.len(),
                        0,
                        0,
                    );

                    self.state = TaskState::Receive;
                }
                TaskState::Receive => {
                    let Some(cqe) = cqe else {
                        panic!("Invalid state")
                    };

                    if cqe.res < 0 {
                        eprintln!("Error whilst Sending {}", Errno::from_raw(-cqe.res));
                    }

                    io_uring_prep_read(
                        sqe,
                        self.fd,
                        buf.receive.as_mut_ptr() as *mut c_void,
                        buf.send.len() as u32,
                        0,
                    );

                    self.state = TaskState::Send;
                }
            }
        }
    }

    pub unsafe fn make_connect(&mut self, sqe: *mut io_uring_sqe) {
        unsafe {
            let settings = get_settings();
            match settings.target {
                SocketAddr::V4(addr) => {
                    let data = Box::pin(sockaddr_in {
                        sin_family: AF_INET as u16,
                        sin_port: addr.port().to_be(),
                        sin_addr: in_addr {
                            s_addr: addr.ip().to_bits().to_be(),
                        },
                        sin_zero: [0; 8],
                    });
                    self.addr = Some(data);

                    let loaded = self.addr.as_mut().unwrap();

                    let pinned = loaded.as_mut().get_mut() as *mut sockaddr_in as *mut sockaddr;
                    io_uring_prep_connect(sqe, self.fd, pinned, size_of::<sockaddr_in>() as u32);
                }
                SocketAddr::V6(adrr) => {
                    let data = Box::pin(sockaddr_in6 {
                        sin6_family: AF_INET6 as u16,
                        sin6_port: adrr.port().to_be(),
                        sin6_flowinfo: 0,
                        sin6_addr: in6_addr {
                            s6_addr: adrr.ip().to_bits().to_be_bytes(),
                        },
                        sin6_scope_id: 0,
                    });
                    self.addr6 = Some(data);

                    let loaded = self.addr6.as_mut().unwrap();

                    let pinned = loaded.as_mut().get_mut() as *mut sockaddr_in6 as *mut sockaddr;
                    io_uring_prep_connect(sqe, self.fd, pinned, size_of::<sockaddr_in6>() as u32);
                }
            }
        }
    }
}
