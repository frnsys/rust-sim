extern crate uuid;
extern crate redis;
extern crate futures;
extern crate futures_cpupool;
extern crate rustc_serialize;
extern crate rmp_serialize;

mod sim;
mod compute;

pub use sim::{Update, State, Simulation};
pub use compute::{Population, Manager, Worker};
