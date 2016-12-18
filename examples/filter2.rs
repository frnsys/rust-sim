#![feature(conservative_impl_trait)]
use std::iter::Filter;
use std::collections::hash_map::{HashMap, Values};

pub trait Agent {}

pub struct Manager<A: Agent> {
    lookup: HashMap<usize, A>,
}

impl<A: Agent> Manager<A> {
    pub fn new() -> Manager<A> {
        Manager { lookup: HashMap::<usize, A>::new() }
    }
    pub fn filter<'a>(&'a self) -> impl Iterator<Item = &'a A> {
        self.lookup.values().filter(|v| true)
    }
}

// ---

fn main() {}
