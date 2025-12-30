use std::{time::Duration};
use crate::settings::get_settings;

#[derive(Debug)]
pub struct Statistics {
    failed_connections: u64,
    wrong_return: u64,
    successful_returns: u64,
    min_nanos: u128,
    max_nanos: u128,
    nanos_spent: u128,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            failed_connections: Default::default(),
            wrong_return: Default::default(),
            successful_returns: Default::default(),
            min_nanos: u128::MAX,
            max_nanos: Default::default(),
            nanos_spent: Default::default(),
        }
    }
}

impl Statistics {
    pub fn merge(&mut self, other: Statistics) {
        self.failed_connections += other.failed_connections;
        self.successful_returns += other.successful_returns;
        self.wrong_return += other.wrong_return;
        self.max_nanos = self.max_nanos.max(other.max_nanos);
        self.min_nanos = self.min_nanos.min(other.min_nanos);
        self.nanos_spent += other.nanos_spent;
    }

    pub fn increment_connect_fail(&mut self) {
        self.failed_connections += 1;
    }

    pub fn increment_wrong_returns(&mut self) {
        self.wrong_return += 1;
    }

    pub fn increment_successful_returns(&mut self) {
        self.successful_returns += 1;
    }

    pub fn new_measurement(&mut self, duration: Duration) {
        let nanos = duration.as_nanos();
        self.min_nanos = self.min_nanos.min(nanos);
        self.max_nanos = self.max_nanos.max(nanos);
        self.nanos_spent += nanos;
    }
}

pub fn print_stats_final(stats: &Statistics) {
    let settings = get_settings();
    println!("BENCHMARK ENDED");
    println!("Failed to connected in {} cases", stats.failed_connections);
    println!(
        "Wrong results have been given in {} cases",
        stats.wrong_return
    );
    println!(
        "Right results have been given in {} cases",
        stats.successful_returns
    );

    println!(
        "Average good pongs per second: {}",
        stats.successful_returns / settings.burn_time.tv_sec as u64
    );

    println!(
        "Maximum time spent {}ms ({}ns)",
        Duration::from_nanos(stats.max_nanos as u64).as_millis(), stats.max_nanos
    );

    println!(
        "Minimum time spent {}ms ({}ns)",
        Duration::from_nanos(stats.min_nanos as u64).as_millis(), stats.min_nanos
    );

    println!("RAW PRINT");
    println!("{stats:#?}");
}
