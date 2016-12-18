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

pub trait Agent: Send + Sync {
    type State: State;
    type Update: Update;
    fn new(state: Self::State) -> Self;
    fn decide(&self) -> ();
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
pub struct Manager<A: Agent> {
    lookup: HashMap<usize, A>,
    last_id: usize,
    updates: HashMap<AgentProxy, Vec<A::Update>>,
}

impl<A: Agent> Manager<A> {
    pub fn new() -> Manager<A> {
        Manager {
            lookup: HashMap::<usize, A>::new(),
            updates: HashMap::<AgentProxy, Vec<A::Update>>::new(),
            last_id: 0,
        }
    }

    /// Spawn a new agent in this manager.
    pub fn spawn(&mut self, state: A::State) {
        let agent = A::new(state);
        self.lookup.insert(self.last_id, agent);
        self.last_id += 1;
    }

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

    /// Queues an update for an agent.
    /// The update is queued locally in the manager until the end of the step.
    pub fn queue_update(&mut self, agent: AgentProxy, update: A::Update) {
        let mut entry = self.updates.entry(agent).or_insert(Vec::new());
        entry.push(update);
    }

    /// Pushes queued updates to agents.
    pub fn push_updates(&mut self) {
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
    pub fn decide(&self) {
        for agent in self.lookup.values() {
            agent.decide();
        }
    }

    /// Calls the `update` method on all local agents.
    /// TODO make this multithreaded
    pub fn update(&mut self) {
        for (_, agent) in self.lookup.iter_mut() {
            agent.update();
        }
    }
}
