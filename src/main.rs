#[macro_use]
extern crate lazy_static;

mod bot;

pub use bot::run;


#[tokio::main]
async fn main() {
    bot::run().await;
}
