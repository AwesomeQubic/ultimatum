use std::{sync::LazyLock, time::Duration};

pub mod settings;
pub mod tasks;
pub mod worker;

fn main() {
    settings::load();
}
