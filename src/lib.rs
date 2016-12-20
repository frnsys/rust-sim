#![feature(unboxed_closures)]
#![feature(conservative_impl_trait)]
extern crate rustc_serialize;

use std::fmt::Debug;
use std::net::SocketAddr;
use std::collections::hash_map::HashMap;
use rustc_serialize::{Decodable, Encodable};

pub trait State: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}
impl<T> State for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Update
    : Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {
}
impl<T> Update for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Manager<A: Agent>: Send + Sync {
    fn new(world: A::World) -> Self;
    fn spawn(&mut self, state: A::State) -> usize;
    fn push_updates(&mut self) -> ();
    fn decide(&mut self) -> ();
    fn update(&mut self) -> ();
    fn world(&self) -> A::World;

    // TODO for now this returns a Box, ideally we could just use `impl trait` but it doesn't work
    // on traits atm
    // fn filter<'a, P>(&'a self, predicate: &'a P) -> impl Iterator<Item = AgentProxy<A>> + 'a
    fn filter<'a, P>(&'a self, predicate: &'a P) -> Box<Iterator<Item = AgentProxy<A>> + 'a>
        where P: Fn(A::State) -> bool;
    fn find<P>(&self, predicate: P) -> Option<AgentProxy<A>> where P: Fn(A::State) -> bool;
    fn get(&self, path: AgentPath) -> Option<AgentProxy<A>>;
}

pub trait World: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}
impl<T> World for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Agent: Send + Sync + Sized {
    type State: State;
    type Update: Update;
    type World: World;
    fn new(state: Self::State) -> Self;
    // TODO we have to pass the world or the manager
    // to the agents during decide
    // ideally this is a clone of:
    // - the world state
    // - agent proxies lookups
    // ideally we can pass the world/manager without
    // requiring locks
    // a RW lock should be ok though; in decide
    // agents will only need read locks, which is ok concurrent
    fn decide<M: Manager<Self>>(&self,
                                world: &Self::World,
                                manager: &M)
                                -> Vec<(AgentProxy<Self>, Self::Update)>;
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

// Managers:
// -- single threaded
// -- multi threaded
// -- multi machine (requires a router)
// the manager needs to be able to sync the world across machines
// but the world also needs access to the local population, e.g.
// to find connected agents in a network
pub struct LocalManager<A: Agent> {
    lookup: HashMap<usize, A>,
    last_id: usize,
    updates: HashMap<AgentPath, Vec<A::Update>>,
    world: A::World,
}

impl<A: Agent> Manager<A> for LocalManager<A> {
    fn new(world: A::World) -> LocalManager<A> {
        LocalManager {
            lookup: HashMap::<usize, A>::new(),
            updates: HashMap::<AgentPath, Vec<A::Update>>::new(),
            last_id: 0,
            world: world,
        }
    }

    fn get(&self, path: AgentPath) -> Option<AgentProxy<A>> {
        match path {
            AgentPath::Local(id) => {
                match self.lookup.get(&id) {
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

    fn world(&self) -> A::World {
        self.world.clone()
    }

    /// Spawn a new agent in this manager.
    fn spawn(&mut self, state: A::State) -> usize {
        let agent = A::new(state);
        self.lookup.insert(self.last_id, agent);
        self.last_id += 1;
        self.last_id
    }

    /// Pushes queued updates to agents.
    fn push_updates(&mut self) {
        for (proxy, updates) in self.updates.iter_mut() {
            match proxy {
                &AgentPath::Local(id) => {
                    match self.lookup.get_mut(&id) {
                        Some(agent) => agent.queue_updates(updates),
                        None => println!("No local agent with id {}", id), // TODO this should probably log an error
                    }
                }
                _ => (),
            }
        }
        self.updates.clear();
    }

    /// Calls the `decide` method on all local agents.
    /// TODO make this multithreaded
    fn decide(&mut self) {
        let mut updates = Vec::new();
        for agent in self.lookup.values() {
            let u = agent.decide(&self.world, self);
            updates.extend(u);
        }

        for (agent, update) in updates {
            let mut entry = self.updates.entry(agent.path).or_insert(Vec::new());
            entry.push(update);
        }
    }

    /// Calls the `update` method on all local agents.
    /// TODO make this multithreaded
    fn update(&mut self) {
        self.push_updates();
        for (_, agent) in self.lookup.iter_mut() {
            agent.update();
        }
    }

    /// TODO when we connect in remote managers, this should probably return a future iterator
    fn filter<'a, P>(&'a self, predicate: &'a P) -> Box<Iterator<Item = AgentProxy<A>> + 'a>
        where P: Fn(A::State) -> bool
    {
        let iter = self.lookup
            .iter()
            .filter(move |&(_, ref a)| predicate(a.state()))
            .map(|(&id, ref a)| {
                AgentProxy {
                    path: AgentPath::Local(id),
                    state: a.state(),
                }
            });
        Box::new(iter)
        // TODO remote lookup
    }

    /// TODO this should probably return a future as well
    fn find<P>(&self, predicate: P) -> Option<AgentProxy<A>>
        where P: Fn(A::State) -> bool
    {
        let res = self.lookup.iter().find(move |&(_, ref a)| predicate(a.state()));
        match res {
            Some((id, a)) => {
                Some(AgentProxy {
                    path: AgentPath::Local(*id),
                    state: a.state(),
                })
            }
            None => None,
        }
        // TODO remote lookup
    }
}
