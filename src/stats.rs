use crate::settings::{self};

#[derive(Default, Debug)]
pub struct Statistics {
    failed_connections: u64,
    wrong_return: u64,
    successful_returns: u64,
}

impl Statistics {
    pub fn merge(&mut self, other: Statistics) {
        self.failed_connections += other.failed_connections;
        self.successful_returns += other.successful_returns;
        self.wrong_return += other.wrong_return;
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
}

pub fn print_stats_final(stats: &Statistics) {
    let settings = settings::get_settings();
    println!("BENCHMARK ENDED");
    println!("Failed to connected in {} cases", stats.failed_connections);
    println!("Wrong results have been given in {} cases", stats.wrong_return);
    println!("Right results have been given in {} cases", stats.successful_returns);
    println!("Average good pongs per second: {}", stats.successful_returns / settings.burn_time.tv_sec as u64);
}
