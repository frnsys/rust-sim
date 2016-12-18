use std::iter::Filter;
use std::collections::hash_map::{HashMap, Values};

pub trait Agent: Send + Sync {}

pub fn filter<A: Agent>(c: HashMap<usize, A>) -> Filter<Values<usize, A>, A> {
    c.values().filter(|&v| true)
}

pub struct MyAgent {}
impl Agent for MyAgent {}

fn main() {
    let lookup = HashMap::<usize, MyAgent>::new();
    let fileted = filter(lookup);
}
