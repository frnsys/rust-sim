extern crate futures;
extern crate futures_cpupool;
extern crate rustc_serialize;

use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use futures::{future, collect, Future};
use futures_cpupool::{CpuPool, CpuFuture};
use std::collections::hash_map::HashMap;
use rustc_serialize::{Decodable, Encodable};

pub trait State: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}
impl<T> State for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Update
    : Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {
}
impl<T> Update for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait World: Decodable + Encodable + Debug + Send + Sync + Clone {}
impl<T> World for T where T: Decodable + Encodable + Debug + Send + Sync + Clone {}

pub trait Manager<S: Simulation>: Send + Sync + 'static {
    fn new(world: S::World) -> Self;
    fn decide(&mut self) -> ();
    fn update(&mut self) -> ();
}

pub trait Simulation: Send + Sync + Clone {
    type State: State;
    type Update: Update;
    type World: World;

    fn new(world: Self::World) -> Self;
    fn apply_update(&self, state: Self::State, update: Self::Update) -> Self::State;
    fn decide<P>(&self,
                 agent: Agent<Self::State>,
                 world: Self::World,
                 population: SharedPopulation<P>)
                 -> Vec<(usize, Self::Update)>;
    fn update(&self, agent: &mut Agent<Self::State>, mut updates: Vec<Self::Update>) -> () {
        let mut state = agent.state.clone();
        for update in updates.drain(..) {
            state = self.apply_update(state, update.clone());
        }
        agent.state = state;
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct Agent<S: State> {
    pub id: usize,
    pub state: S,
}

pub trait Population<S: State> {
    fn spawn(&mut self, state: S) -> usize;
    fn get(&self, id: usize) -> Option<Agent<S>>;
}

pub type SharedPopulation<Population> = Arc<RwLock<Population>>;

#[derive(Debug, Clone)]
pub struct LocalPopulation<S: State> {
    last_id: usize,
    pub agents: HashMap<usize, Agent<S>>,
}

impl<S: State> LocalPopulation<S> {
    pub fn new() -> LocalPopulation<S> {
        LocalPopulation {
            last_id: 0,
            agents: HashMap::<usize, Agent<S>>::new(),
        }
    }
}

impl<S: State> Population<S> for LocalPopulation<S> {
    fn spawn(&mut self, state: S) -> usize {
        let agent = Agent {
            state: state,
            id: self.last_id,
        };
        self.agents.insert(self.last_id, agent);
        self.last_id += 1;
        self.last_id - 1
    }

    fn get(&self, id: usize) -> Option<Agent<S>> {
        match self.agents.get(&id) {
            Some(a) => {
                Some(Agent {
                    id: a.id,
                    state: a.state.clone(),
                })
            }
            None => None,
        }
    }
}

pub struct LocalManager<S: Simulation> {
    updates: HashMap<usize, Vec<S::Update>>,
    world: S::World,
    pub population: SharedPopulation<LocalPopulation<S::State>>,
    simulation: S,
}

impl<S: Simulation + 'static> Manager<S> for LocalManager<S> {
    fn new(world: S::World) -> LocalManager<S> {
        LocalManager {
            updates: HashMap::<usize, Vec<S::Update>>::new(),
            world: world.clone(),
            population: Arc::new(RwLock::new(LocalPopulation::new())),
            simulation: S::new(world),
        }
    }

    /// Calls the `decide` method on all agents.
    fn decide(&mut self) {
        let mut futs = Vec::new();
        let pool = CpuPool::new_num_cpus();
        let world = self.world.clone();
        let pop = self.population.read().unwrap();
        for agent in pop.agents.values() {
            let pop = self.population.clone();
            let agent = agent.clone();
            let world = world.clone();
            let sim = self.simulation.clone();
            let f: CpuFuture<Vec<(usize, S::Update)>, ()> =
                pool.spawn(future::lazy(move || future::finished(sim.decide(agent, world, pop))));
            futs.push(f);
        }

        let f = collect(futs);
        let updates_list = f.wait().unwrap();
        for updates in updates_list {
            for (id, update) in updates {
                let mut entry = self.updates.entry(id).or_insert(Vec::new());
                entry.push(update);
            }
        }
    }

    /// Calls the `update` method on all agents.
    fn update(&mut self) {
        let mut population = self.population.write().unwrap();
        for (id, updates) in self.updates.drain() {
            match population.agents.get_mut(&id) {
                Some(agent) => self.simulation.update(agent, updates),
                None => println!("No agent with id {}", id), // TODO this should probably log an error
            }
        }
        self.updates.clear();
    }
}
