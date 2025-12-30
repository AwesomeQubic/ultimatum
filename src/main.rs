
pub mod settings;
pub mod stats;
pub mod tasks;
pub mod uring;
pub mod worker;

fn main() {
    let parsed = settings::load();
    if parsed.debug {
        println!("{:#?}", parsed)
    }

    worker::burn();
}
