use std::{
    net::SocketAddr,
    os::{fd::RawFd, raw::c_void},
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
};

use libc::{in6_addr, in_addr, sockaddr_in, sockaddr_in6, AF_INET};
use liburing_rs::*;
use rand_core::RngCore;
use wyrand::WyRand;

use crate::settings::{Protocol, Settings};

static SEEDS: AtomicU64 = AtomicU64::new(0);

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

    //Using *mut io_uring tho NonNull<io_uring> would be probably more idiomatic
    //But its less readable
    pub fn setup(io: *mut io_uring, connections: usize, settings: &Settings) -> ThreadLocalTasking {
        let len = connections * TASK_BUF;
        let mut mapped = memmap2::MmapMut::map_anon(len).expect("Could not map memory");

        unsafe {
            let io_vec = iovec {
                iov_base: mapped.as_mut_ptr() as *mut c_void,
                iov_len: len,
            };

            assert!(
                io_uring_register_buffers(io, &raw const io_vec, 1) >= 0,
                "Could not register buffer"
            );

            let mut tasks = Vec::with_capacity(connections);
            for i in 0..connections {
                tasks.push(Task {
                    index: i,
                    fd: 0,
                    rand: WyRand::new(SEEDS.fetch_add(1, Ordering::Relaxed)),
                    state: TaskState::NewSock,
                    addr: None,
                    addr6: None,
                });
            }

            for ele in tasks.iter_mut() {
                let buf = ele.index;
                let array = &mut mapped[buf * TASK_BUF..buf * TASK_BUF + TASK_BUF];

                let (send, receive) = array.split_at_mut(4096);

                let mut buf = TaskBuf { send, receive };

                ele.progress(settings, None, io, &mut buf);
            }

            ThreadLocalTasking {
                tasks: tasks.into_boxed_slice(),
                memory: mapped,
            }
        }
    }

    pub fn progress(&mut self, io: *mut io_uring, settings: &Settings, cqe: io_uring_cqe) {
        let index = cqe.user_data as usize;

        let array = &mut self.memory[index * TASK_BUF..index * TASK_BUF + TASK_BUF];

        let (send, receive) = array.split_at_mut(4096);

        let mut buf = TaskBuf { send, receive };

        self.tasks[index].progress(settings, Some(cqe), io, &mut buf);
    }
}

struct Task {
    index: usize,
    fd: RawFd,
    rand: WyRand,
    state: TaskState,

    //Adresses
    addr: Option<Pin<Box<sockaddr_in>>>,
    addr6: Option<Pin<Box<sockaddr_in6>>>,
}

#[derive(Default)]
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
        settings: &Settings,
        cqe: Option<io_uring_cqe>,
        ring: *mut io_uring,
        buf: &mut TaskBuf<'_>,
    ) {
        unsafe {
            let mut sqe = io_uring_get_sqe(ring);

            if sqe.is_null() {
                io_uring_submit(ring);
                sqe = io_uring_get_sqe(ring);

                if sqe.is_null() {
                    panic!("Could not find a place to progress a task");
                }
            }

            match self.state {
                TaskState::NewSock => {
                    let sock_type = match settings.proto {
                        Protocol::Tcp => SOCK_STREAM,
                        Protocol::Udp => SOCK_DGRAM,
                    };

                    let domain = match settings.target {
                        SocketAddr::V4(_) => AF_INET as i32,
                        SocketAddr::V6(_) => AF_INET6 as i32,
                    };
                    io_uring_prep_socket_direct_alloc(sqe, domain, sock_type as i32, 0, 0);
                    io_uring_sqe_set_data64(sqe, self.index as u64);

                    self.state = TaskState::Connect;
                }
                TaskState::Connect => {
                    let Some(cqe) = cqe else {
                        panic!("Invalid state")
                    };

                    if cqe.res < 0 {
                        panic!("Could not create socket");
                    }

                    self.fd = cqe.res;

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

                            let pinned =
                                loaded.as_mut().get_mut() as *mut sockaddr_in as *mut sockaddr;
                            io_uring_prep_connect(
                                sqe,
                                self.fd,
                                pinned,
                                size_of::<sockaddr_in>() as u32,
                            );
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

                            let pinned =
                                loaded.as_mut().get_mut() as *mut sockaddr_in6 as *mut sockaddr;
                            io_uring_prep_connect(
                                sqe,
                                self.fd,
                                pinned,
                                size_of::<sockaddr_in6>() as u32,
                            );
                        }
                    }

                    io_uring_sqe_set_data64(sqe, self.index as u64);

                    self.state = TaskState::Setup;
                }
                TaskState::Setup => {
                    let Some(cqe) = cqe else {
                        panic!("Invalid state")
                    };

                    self.addr = None;
                    self.addr6 = None;

                    self.rand.fill_bytes(buf.send);

                    io_uring_prep_send_zc_fixed(
                        sqe,
                        self.fd,
                        buf.send.as_ptr() as *const c_void,
                        buf.send.len(),
                        0,
                        0,
                        0,
                    );
                    io_uring_sqe_set_data64(sqe, self.index as u64);

                    self.state = TaskState::Receive;
                }
                TaskState::Send => {
                    let Some(cqe) = cqe else {
                        panic!("Invalid state")
                    };

                    if buf.receive != buf.send {
                        panic!("AWA");
                    }

                    self.rand.fill_bytes(buf.send);

                    io_uring_prep_send_zc_fixed(
                        sqe,
                        self.fd,
                        buf.send.as_ptr() as *const c_void,
                        buf.send.len(),
                        0,
                        0,
                        0,
                    );
                    io_uring_sqe_set_data64(sqe, self.index as u64);

                    self.state = TaskState::Receive;
                }
                TaskState::Receive => {
                    let Some(cqe) = cqe else {
                        panic!("Invalid state")
                    };

                    if cqe.flags & IORING_CQE_F_MORE != 0 {
                        return;
                    }

                    io_uring_prep_read_fixed(
                        sqe,
                        self.fd,
                        buf.send.as_mut_ptr() as *mut c_void,
                        buf.send.len() as u32,
                        0,
                        0,
                    );
                    io_uring_sqe_set_data64(sqe, self.index as u64);

                    self.state = TaskState::Send;
                }
            }
        }
    }
}
