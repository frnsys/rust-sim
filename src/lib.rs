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
    fn spawn(&mut self, state: A::State) -> ();
    fn queue_update(&mut self, agent: AgentProxy, update: A::Update) -> ();
    fn push_updates(&mut self) -> ();
    fn decide(&self) -> ();
    fn update(&mut self) -> ();
    fn world(&self) -> A::World;
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
    fn decide<M: Manager<Self>>(&self, world: &Self::World, manager: &M) -> ();
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

#[derive(PartialEq, Eq, Hash)]
pub enum AgentProxy {
    Local(usize),
    Remote(usize, SocketAddr), // TODO
}

// TODO agents need some way of finding other agents
// and also querying a world state
// these two don't necessarily be the same (i.e. we can have distinct Manager and World objects)
// worlds can be local to the node and updated/synchronized between simulation steps
// this never gives out the actual agent, just a proxy
// proxies should be cloneable/copiable
// so perhaps this just keeps a hashmap to LOCAL proxies
// if a local id is not found, query the leader manager?
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
    updates: HashMap<AgentProxy, Vec<A::Update>>,
    world: A::World,
}

impl<A: Agent> Manager<A> for LocalManager<A> {
    fn new(world: A::World) -> LocalManager<A> {
        LocalManager {
            lookup: HashMap::<usize, A>::new(),
            updates: HashMap::<AgentProxy, Vec<A::Update>>::new(),
            last_id: 0,
            world: world,
        }
    }

    fn world(&self) -> A::World {
        self.world.clone()
    }

    /// Spawn a new agent in this manager.
    fn spawn(&mut self, state: A::State) {
        let agent = A::new(state);
        self.lookup.insert(self.last_id, agent);
        self.last_id += 1;
    }

    /// Queues an update for an agent.
    /// The update is queued locally in the manager until the end of the step.
    fn queue_update(&mut self, agent: AgentProxy, update: A::Update) {
        let mut entry = self.updates.entry(agent).or_insert(Vec::new());
        entry.push(update);
    }

    /// Pushes queued updates to agents.
    fn push_updates(&mut self) {
        for (proxy, updates) in self.updates.iter_mut() {
            match proxy {
                &AgentProxy::Local(id) => {
                    match self.lookup.get_mut(&id) {
                        Some(agent) => agent.queue_updates(updates),
                        None => println!("TODO"),
                    }
                }
                &AgentProxy::Remote(id, addr) => {
                    // TODO submit remote update
                }
            }
        }
        self.updates.clear();
    }

    /// Calls the `decide` method on all local agents.
    /// TODO make this multithreaded
    fn decide(&self) {
        for agent in self.lookup.values() {
            agent.decide(&self.world, self);
        }
    }

    /// Calls the `update` method on all local agents.
    /// TODO make this multithreaded
    fn update(&mut self) {
        for (_, agent) in self.lookup.iter_mut() {
            agent.update();
        }
    }
}

impl<A: Agent> LocalManager<A> {
    /// TODO when we connect in remote managers, this should probably return a future iterator
    pub fn filter<'a, P>(&'a self, predicate: &'a P) -> impl Iterator<Item = AgentProxy> + 'a
        where P: Fn(A::State) -> bool
    {
        self.lookup
            .iter()
            .filter(move |&(_, ref a)| predicate(a.state()))
            .map(|(&id, _)| AgentProxy::Local(id))
        // TODO remote lookup
    }

    /// TODO this should probably return a future as well
    pub fn find<P>(&self, predicate: P) -> Option<&A>
        where P: Fn(A::State) -> bool
    {
        self.lookup.values().find(move |&a| predicate(a.state()))
        // TODO remote lookup
    }
}
