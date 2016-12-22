#![feature(unboxed_closures)]
#![feature(conservative_impl_trait)]
extern crate rand;
extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
extern crate rmp_serialize;
extern crate rustc_serialize;

mod proto;
mod router;

use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use futures::{future, collect, Future};
use futures_cpupool::{CpuPool, CpuFuture};
use rand::{thread_rng, Rng, sample};
use std::collections::hash_map::HashMap;
use rustc_serialize::{Decodable, Encodable};

pub trait State: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}
impl<T> State for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Update
    : Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {
}
impl<T> Update for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Manager<A: Agent>: Send + Sync + 'static {
    fn new(world: A::World) -> Self;
    fn push_updates(&mut self) -> ();
    fn setup(&mut self) -> ();
    fn decide(&mut self) -> ();
    fn update(&mut self) -> ();
    fn world(&self) -> A::World;
}

pub trait World: Decodable + Encodable + Debug + Send + Sync + Clone {}
impl<T> World for T where T: Decodable + Encodable + Debug + Send + Sync + Clone {}

pub trait Agent: Send + Sync + Sized + Clone {
    type State: State;
    type Update: Update;
    type World: World;
    fn new(state: Self::State, id: usize) -> Self;
    fn id(&self) -> usize;
    fn setup(&mut self, world: &Self::World) -> ();
    fn decide(&self,
              world: Self::World,
              population: SharedPopulation<Self>)
              -> Vec<(AgentPath, Self::Update)>;
    fn state(&self) -> Self::State;
    fn set_state(&mut self, state: Self::State) -> ();
    fn updates(&self) -> &Vec<Self::Update>;
    fn queue_updates(&mut self, updates: &mut Vec<Self::Update>) -> ();
    fn apply_update(&self, state: Self::State, update: Self::Update) -> Self::State;
    fn update(&mut self) -> () {
        let mut state = self.state();
        for update in self.updates() {
            state = self.apply_update(state, update.clone());
        }
        // TODO reset updates
        self.set_state(state);
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum AgentPath {
    Local(usize),
    Remote(usize, SocketAddr),
}

#[derive(Debug, PartialEq, Clone)]
pub struct AgentProxy<A: Agent> {
    pub path: AgentPath,
    pub state: A::State,
}

#[derive(Debug, Clone)]
pub struct Population<A: Agent> {
    last_id: usize,
    pub local: HashMap<usize, A>,
}

impl<A: Agent> Population<A> {
    pub fn new() -> Population<A> {
        Population {
            last_id: 0,
            local: HashMap::<usize, A>::new(),
        }
    }

    pub fn spawn(&mut self, state: A::State) -> usize {
        let agent = A::new(state, self.last_id);
        self.local.insert(self.last_id, agent);
        self.last_id += 1;
        self.last_id - 1
    }

    pub fn get(&self, path: AgentPath) -> Option<AgentProxy<A>> {
        match path {
            AgentPath::Local(id) => {
                match self.local.get(&id) {
                    Some(a) => {
                        Some(AgentProxy {
                            path: path,
                            state: a.state(),
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    // TODO for now this returns a Box, ideally we could just use `impl trait` but it doesn't work
    // on traits atm
    // fn filter<'a, P>(&'a self, predicate: &'a P) -> impl Iterator<Item = AgentProxy<A>> + 'a
    /// TODO when we connect in remote managers, this should probably return a future iterator
    pub fn filter<'a, P>(&'a self, predicate: &'a P) -> Box<Iterator<Item = AgentProxy<A>> + 'a>
        where P: Fn(A::State) -> bool
    {
        let iter = self.local
            .iter()
            .filter(move |&(_, ref a)| predicate(a.state()))
            .map(|(&id, ref a)| {
                AgentProxy {
                    path: AgentPath::Local(id),
                    state: a.state(),
                }
            });
        Box::new(iter)
        // TODO remote local
    }

    // TODO problem with these is they may sample themselves
    pub fn sample(&self, n: usize) -> Vec<AgentProxy<A>> {
        let mut rng = thread_rng();
        let iter = self.local.iter().map(|(&id, ref a)| {
            AgentProxy {
                path: AgentPath::Local(id),
                state: a.state(),
            }
        });
        sample(&mut rng, iter, n)
    }

    pub fn sample_by<'a, P>(&'a self,
                            predicate: &'a P,
                            n: usize)
                            -> Box<Iterator<Item = AgentProxy<A>> + 'a>
        where P: Fn(A::State) -> f64
    {
        // hashmap iteration order is arbitrary
        // so no need to shuffle?
        let mut rng = rand::thread_rng();
        let iter = self.local
            .iter()
            .filter(move |&(_, ref a)| {
                let prob = predicate(a.state());
                let roll: f64 = rng.gen();
                roll <= prob
            });

        // TODO this doesn't work
        // let iter = if n > 0 { iter.take(n) } else { iter };

        let iter = iter.take(n).map(|(&id, ref a)| {
            AgentProxy {
                path: AgentPath::Local(id),
                state: a.state(),
            }
        });
        Box::new(iter)
    }

    /// TODO this should probably return a future as well
    pub fn find<P>(&self, predicate: P) -> Option<AgentProxy<A>>
        where P: Fn(A::State) -> bool
    {
        let res = self.local.iter().find(move |&(_, ref a)| predicate(a.state()));
        match res {
            Some((id, a)) => {
                Some(AgentProxy {
                    path: AgentPath::Local(*id),
                    state: a.state(),
                })
            }
            None => None,
        }
        // TODO remote local
    }
}

pub type SharedPopulation<A: Agent> = Arc<RwLock<Population<A>>>;

// Managers:
// -- single threaded
// -- multi threaded
// -- multi machine (requires a router)
// the manager needs to be able to sync the world across machines
// but the world also needs access to the local population, e.g.
// to find connected agents in a network
pub struct LocalManager<A: Agent> {
    updates: HashMap<AgentPath, Vec<A::Update>>,
    world: A::World,
    pub population: SharedPopulation<A>,
}

impl<A: Agent + 'static> Manager<A> for LocalManager<A> {
    fn new(world: A::World) -> LocalManager<A> {
        LocalManager {
            updates: HashMap::<AgentPath, Vec<A::Update>>::new(),
            world: world,
            population: Arc::new(RwLock::new(Population::new())),
        }
    }

    fn world(&self) -> A::World {
        self.world.clone()
    }

    /// Pushes queued updates to agents.
    fn push_updates(&mut self) {
        let mut population = self.population.write().unwrap();
        for (proxy, updates) in self.updates.iter_mut() {
            match proxy {
                &AgentPath::Local(id) => {
                    match population.local.get_mut(&id) {
                        Some(agent) => agent.queue_updates(updates),
                        None => println!("No local agent with id {}", id), // TODO this should probably log an error
                    }
                }
                _ => (),
            }
        }
        self.updates.clear();
    }

    /// Calls the `setup` method on all local agents.
    fn setup(&mut self) {
        let mut population = self.population.write().unwrap();
        for agent in population.local.values_mut() {
            agent.setup(&self.world);
        }
    }

    /// Calls the `decide` method on all local agents.
    fn decide(&mut self) {
        let mut futs = Vec::new();
        let pool = CpuPool::new_num_cpus();
        let world = self.world.clone();
        let pop = self.population.read().unwrap();
        for agent in pop.local.values() {
            let pop = self.population.clone();
            let agent = agent.clone();
            let world = world.clone();
            let f: CpuFuture<Vec<(AgentPath, A::Update)>, ()> =
                pool.spawn(future::lazy(move || future::finished(agent.decide(world, pop))));
            futs.push(f);
        }

        let f = collect(futs);
        let updates_list = f.wait().unwrap();
        for updates in updates_list {
            for (path, update) in updates {
                let mut entry = self.updates.entry(path).or_insert(Vec::new());
                entry.push(update);
            }
        }
    }

    /// Calls the `update` method on all local agents.
    fn update(&mut self) {
        self.push_updates();
        let mut population = self.population.write().unwrap();
        for (_, agent) in population.local.iter_mut() {
            agent.update();
        }
    }
}
