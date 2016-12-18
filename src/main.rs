#![feature(unboxed_closures)]
#![feature(conservative_impl_trait)]
extern crate rustc_serialize;

use std::fmt::Debug;
use std::iter::Filter;
use std::marker::PhantomData;
use std::collections::hash_map::{HashMap, Values};
use rustc_serialize::{Decodable, Encodable};

pub trait State: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}
impl<T> State for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Update
    : Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {
}
impl<T> Update for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Agent: Send + Sync {
    type State: State;
    type Update: Update;
    fn new(state: Self::State) -> Self;
    fn decide(&self) -> ();
    fn state(&self) -> Self::State;
    fn set_state(&mut self, state: Self::State) -> ();
    fn updates(&self) -> Vec<Self::Update>;
    fn apply_update(&self, state: Self::State, update: Self::Update) -> Self::State;
    fn update(&mut self) -> () {
        let mut state = self.state();
        for update in self.updates() {
            state = self.apply_update(state, update);
        }
        self.set_state(state);
    }
}

pub enum AgentProxy<A: Agent> {
    Local(A),
    Remote(A), // TODO
}

// TODO agents need some way of finding other agents
// and also querying a world state
// these two don't necessarily be the same (i.e. we can have distinct Manager and World objects)
// worlds can be local to the node and updated/synchronized between simulation steps
// this never gives out the actual agent, just a proxy
// proxies should be cloneable/copiable
// so perhaps this just keeps a hashmap to LOCAL proxies
// if a local id is not found, query the leader manager?
pub struct Manager<A: Agent> {
    lookup: HashMap<usize, A>,
    last_id: usize,
}

impl<A: Agent> Manager<A> {
    pub fn new() -> Manager<A> {
        Manager {
            lookup: HashMap::<usize, A>::new(),
            last_id: 0,
        }
    }
    pub fn spawn(&mut self, state: A::State) {
        let agent = A::new(state);
        self.lookup.insert(self.last_id, agent);
        self.last_id += 1;
    }
    pub fn filter<'a, P>(&'a self, predicate: &'a P) -> impl Iterator<Item = &'a A> + 'a
        where P: Fn(A::State) -> bool
    {
        self.lookup.values().filter(move |&a| predicate(a.state()))
        // TODO remote lookup
    }
    // pub fn find<F>(&self, predicate: F) TODO short circuit when found
    pub fn submit_update(agent: AgentProxy<A>, update: A::Update) {
        // TODO when sending to remote agents, we can just locally queue all outgoing updates
        // and send them all at the end of the simulation step
    }

    // TODO
    pub fn decide() {}
    pub fn update() {}
}

// ---

// TODO may be possible to even do an enum of states? that's how you could represent different
// agent types
#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct MyState {
    name: String,
    health: usize,
}

pub struct MyAgent {
    state: MyState,
}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub enum MyUpdate {
    ChangeName(String),
    ChangeHealth(usize),
}

impl Agent for MyAgent {
    type State = MyState;
    type Update = MyUpdate;
    fn new(state: MyState) -> MyAgent {
        MyAgent { state: state }
    }
    fn decide(&self) -> () {}
    fn state(&self) -> MyState {
        self.state.clone()
    }
    fn set_state(&mut self, state: MyState) -> () {
        self.state = state;
    }
    fn updates(&self) -> Vec<MyUpdate> {
        let mut v = Vec::<MyUpdate>::new();
        v.push(MyUpdate::ChangeName("bar".to_string()));
        v
    }
    fn apply_update(&self, state: MyState, update: MyUpdate) -> Self::State {
        match update {
            MyUpdate::ChangeName(name) => {
                MyState {
                    name: name,
                    health: state.health,
                }
            }
            MyUpdate::ChangeHealth(health) => {
                MyState {
                    name: state.name,
                    health: state.health + health,
                }
            }
        }
    }
}

fn main() {
    let mut agent = MyAgent {
        state: MyState {
            name: "hello".to_string(),
            health: 0,
        },
    };
    let update = MyUpdate::ChangeHealth(10);
    println!("{:?}", agent.state);
    agent.state = agent.apply_update(agent.state(), update);
    println!("{:?}", agent.state);
}
