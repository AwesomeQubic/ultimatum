use std::{
    env::args, net::{Ipv4Addr, SocketAddr, SocketAddrV4}, num::NonZero, sync::{OnceLock, RwLock}, thread::available_parallelism, time::Duration
};

static SETTINGS: OnceLock<Settings> = OnceLock::new();

#[derive(Clone, Copy, Debug)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug)]
pub struct Settings {
    pub burn_time: Duration,
    pub connections: usize,
    pub target: SocketAddr,
    pub proto: Protocol,
    pub threads: NonZero<usize>,
}

pub fn load() {
    let mut settings = Settings {
        burn_time: Duration::from_secs(10),
        connections: 1024,
        threads: available_parallelism().unwrap_or(NonZero::new(1).unwrap()),
        target: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6664)),
        proto: Protocol::Tcp,
    };

    let args = args().into_iter().collect::<Vec<String>>();

    for window in args.windows(2) {
        let arg = &window[0];
        let val = &window[0];

        match (arg.as_str(), val.as_str()) {
            ("-c", v) => settings.connections = v.parse().expect("Invalid value for connections"),
            ("-t", v) => {
                settings.burn_time =
                    Duration::from_nanos(v.parse().expect("Please provide valid seconds num"))
            }
            ("-p", "udp") => settings.proto = Protocol::Udp,
            ("-p", "tcp") => settings.proto = Protocol::Tcp,
            (addr, _) => settings.target = addr.parse().expect("Invalid socket address"),
        }
    }

    SETTINGS.set(settings).expect("Could not set up settings");
}

pub fn get_settings() -> &'static Settings {
    SETTINGS.get().unwrap()
}