use std::{
    env::args,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    num::NonZero,
    sync::OnceLock,
    thread::available_parallelism,
    time::Duration,
};

use liburing_rs::__kernel_timespec;

static SETTINGS: OnceLock<Settings> = OnceLock::new();

#[derive(Clone, Copy, Debug)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug)]
pub struct Settings {
    pub burn_time: __kernel_timespec,
    pub connections: usize,
    pub target: SocketAddr,
    pub proto: Protocol,
    pub threads: NonZero<usize>,
    pub debug: bool,
}

impl Settings {
    pub fn connections_per_thread(&self) -> usize {
        self.connections.div_ceil(self.threads.get())
    }
}

pub fn load() -> &'static Settings {
    let mut settings = Settings {
        burn_time: __kernel_timespec::from(Duration::from_secs(10)),
        connections: 1024,
        threads: available_parallelism().unwrap_or(NonZero::new(1).unwrap()),
        target: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6664)),
        proto: Protocol::Udp,
        debug: false,
    };

    let mut args = args().collect::<Vec<String>>();

    args.remove(0);

    for window in args.chunks(2) {
        let arg = &window[0];
        let val = &window[1];

        println!("{arg}={val}");

        match (arg.as_str(), val.as_str()) {
            ("-c", v) => settings.connections = v.parse().expect("Invalid value for connections"),
            ("-b", v) => {
                settings.burn_time = __kernel_timespec::from(Duration::from_secs(
                    v.parse().expect("Please provide valid seconds num"),
                ))
            }
            ("-t", v) => settings.threads = v.parse().expect("Could not parse threads"),
            ("-p", "udp") => settings.proto = Protocol::Udp,
            ("-p", "tcp") => settings.proto = Protocol::Tcp,
            ("--debug", "yes") => settings.debug = true,
            (addr, _) => settings.target = addr.parse().expect("Invalid socket address"),
        }
    }

    SETTINGS.set(settings).expect("Could not set up settings");
    unsafe { SETTINGS.get().unwrap_unchecked() }
}

#[inline]
pub fn get_settings() -> &'static Settings {
    SETTINGS.get().unwrap()
}
