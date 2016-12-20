extern crate sim;
extern crate time;
extern crate rand;
extern crate rustc_serialize;

use time::PreciseTime;
use rand::{thread_rng, Rng};
use sim::{Agent, Manager, LocalManager, AgentProxy, AgentPath};

// TODO may be possible to even do an enum of states? that's how you could represent different
// agent types
#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct State {
    altruism: f64,
    frugality: f64,
}

#[derive(Debug)]
pub struct Person {
    id: usize,
    state: State,
    updates: Vec<Update>,
}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct World {}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub enum Update {
    Imitate(State),
}

impl Agent for Person {
    type State = State;
    type Update = Update;
    type World = World;
    fn new(state: State, id: usize) -> Person {
        Person {
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

        // this doesn't scale well with agent population
        let others = manager.sample(1);
        let ref other = others[0];

        updates.push((AgentPath::Local(self.id), Update::Imitate(other.state.clone())));
        updates
    }
    fn state(&self) -> State {
        self.state.clone()
    }
    fn set_state(&mut self, state: State) -> () {
        self.state = state;
    }
    fn updates(&self) -> &Vec<Update> {
        &self.updates
    }
    fn queue_updates(&mut self, updates: &mut Vec<Update>) -> () {
        self.updates.append(updates);
    }
    fn apply_update(&self, state: State, update: Update) -> Self::State {
        match update {
            Update::Imitate(state) => {
                let diff_altruism = state.altruism - self.state.altruism;
                let diff_frugality = state.frugality - self.state.frugality;
                State {
                    altruism: state.altruism + diff_altruism * 0.01,
                    frugality: state.frugality + diff_frugality * 0.01,
                }
            }
        }
    }
}

fn main() {
    let state = State {
        altruism: 0.5,
        frugality: 0.5,
    };
    let state2 = State {
        altruism: 1.,
        frugality: 1.,
    };

    let world = World {};
    let mut manager = LocalManager::<Person>::new(world);

    let mut rng = thread_rng();
    for i in 0..500 {
        let roll: f64 = rng.gen();
        let s = if roll <= 0.5 {
            state.clone()
        } else {
            state2.clone()
        };
        manager.spawn(s);
    }
    for i in 0..10 {
        let start = PreciseTime::now();
        manager.decide();
        manager.update();
        let end = PreciseTime::now();
        println!("step took: {}", start.to(end));
    }
    let a = manager.get(AgentPath::Local(1)).unwrap();
    println!("{:?}", a);
    let a = manager.get(AgentPath::Local(0)).unwrap();
    println!("{:?}", a);
    println!("ok");
}
