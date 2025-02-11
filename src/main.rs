#[allow(unused_imports)]
#[allow(dead_code)]
#[macro_use(info, debug, error, warn)]
extern crate tracing;

fn main() {
  tracing_subscriber::fmt().without_time().init();
  info!("start");
}
