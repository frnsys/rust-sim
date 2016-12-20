extern crate sim;
extern crate rustc_serialize;

use sim::{Agent, Manager, LocalManager, State, AgentProxy, AgentPath};

// TODO may be possible to even do an enum of states? that's how you could represent different
// agent types
#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct MyState {
    name: String,
    health: usize,
}

#[derive(Debug)]
pub struct MyAgent {
    id: usize,
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
    fn new(state: MyState, id: usize) -> MyAgent {
        MyAgent {
            id: id,
            state: state,
            updates: Vec::new(),
        }
    }
    fn id(&self) -> usize {
        self.id
    }
    fn decide<M: Manager<Self>>(&self,
                                world: &Self::World,
                                manager: &M)
                                -> Vec<(AgentPath, Self::Update)> {
        let mut updates = Vec::new();
        match self.state.name.as_ref() {
            "hello" => {
                println!("my name is hello");
                match manager.find(|s| s.name == "goodbye") {
                    Some(a) => {
                        println!("other name: {:?}", a);
                        updates.push((a.path, MyUpdate::ChangeHealth(12)));
                    }
                    None => println!("not found"),
                }

            }
            "goodbye" => println!("my name is goodbye"),
            _ => println!("my name is unknown"),
        }
        updates
    }
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
    let health = 10;
    let state = MyState {
        name: "hello".to_string(),
        health: 0,
    };
    let state2 = MyState {
        name: "goodbye".to_string(),
        health: health,
    };
    let world = MyWorld { weather: "sunny".to_string() };
    let mut manager = LocalManager::<MyAgent>::new(world);
    manager.spawn(state.clone());
    manager.spawn(state2.clone());
    manager.decide();
    manager.update();
    let a = manager.get(AgentPath::Local(1)).unwrap();
    println!("{:?}", a);
    println!("ok");
    assert_eq!(a.state.health, health + 12);
}
