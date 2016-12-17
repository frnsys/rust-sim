extern crate rustc_serialize;

use std::fmt::Debug;
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
