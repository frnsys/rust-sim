extern crate sim;
extern crate rustc_serialize;

use sim::{Agent, Manager};

// TODO may be possible to even do an enum of states? that's how you could represent different
// agent types
#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct MyState {
    name: String,
    health: usize,
}

pub struct MyAgent {
    state: MyState,
    updates: Vec<MyUpdate>,
}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct MyWorld {
    weather: String,
}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub enum MyUpdate {
    ChangeName(String),
    ChangeHealth(usize),
}

impl Agent for MyAgent {
    type State = MyState;
    type Update = MyUpdate;
    type World = MyWorld;
    fn new(state: MyState) -> MyAgent {
        MyAgent {
            state: state,
            updates: Vec::new(),
        }
    }
    fn decide<M: Manager<Self>>(&self, world: &Self::World, manager: &M) -> () {}
    fn state(&self) -> MyState {
        self.state.clone()
    }
    fn set_state(&mut self, state: MyState) -> () {
        self.state = state;
    }
    fn updates(&self) -> &Vec<MyUpdate> {
        &self.updates
    }
    fn queue_updates(&mut self, updates: &mut Vec<MyUpdate>) -> () {
        self.updates.append(updates);
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
    let state = MyState {
        name: "hello".to_string(),
        health: 0,
    };
    let mut agent = MyAgent::new(state);
    let update = MyUpdate::ChangeHealth(10);
    println!("{:?}", agent.state);
    agent.state = agent.apply_update(agent.state(), update);
    println!("{:?}", agent.state);
}
