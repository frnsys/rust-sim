extern crate sim;
extern crate time;
extern crate rand;
extern crate rustc_serialize;

use time::PreciseTime;
use rand::{thread_rng, Rng, sample};
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
    friends: Vec<AgentPath>,
}

#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct World {}

impl World {
    pub fn new() -> World {
        World {}
    }
}

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
            friends: Vec::new(),
        }
    }
    fn id(&self) -> usize {
        self.id
    }
    fn setup(&mut self, world: &Self::World) -> () {}
    fn decide<M: Manager<Self>>(&self,
                                world: &Self::World,
                                manager: &M)
                                -> Vec<(AgentPath, Self::Update)> {
        let mut updates = Vec::new();

        for friend_path in self.friends.iter() {
            let friend = match manager.get(friend_path.clone()) {
                Some(a) => a,
                None => panic!("couldnt find friend: {:?}", friend_path),
            };
            updates.push((AgentPath::Local(self.id), Update::Imitate(friend.state.clone())));
        }
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
    let start = PreciseTime::now();

    let state = State {
        altruism: 0.5,
        frugality: 0.5,
    };
    let state2 = State {
        altruism: 1.,
        frugality: 1.,
    };

    let world = World::new();
    let mut manager = LocalManager::<Person>::new(world);

    let mut ids = Vec::new();
    let mut rng = thread_rng();
    for i in 0..1000 {
        let roll: f64 = rng.gen();
        let s = if roll <= 0.5 {
            state.clone()
        } else {
            state2.clone()
        };
        let id = manager.spawn(s);
        ids.push(id);
    }

    // assign friends
    let n_friends = 10;
    for id in ids.clone() {
        match manager.lookup.get_mut(&id) {
            Some(a) => {
                let friends = sample(&mut rng, ids.clone(), n_friends);
                for id in friends {
                    a.friends.push(AgentPath::Local(id));
                }
            }
            _ => (),
        }
    }

    manager.setup();

    let end = PreciseTime::now();
    println!("setup took: {}", start.to(end));

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
