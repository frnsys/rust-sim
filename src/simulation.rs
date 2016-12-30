use uuid::Uuid;
use std::fmt::Debug;
use rustc_serialize::{Decodable, Encodable};

pub trait State: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}
impl<T> State for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Update
    : Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {
}
impl<T> Update for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

/// Agents are just structures containing a unique id and a state.
#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct Agent<S: State> {
    pub id: Uuid,
    pub state: S,
}

/// This trait's implementation defines the main logic of a simulation.
/// A single simulation step consists of two synchronized phases:
/// 1. `decide`: this is a _read-only_ phase where agents decide on what _updates_ to apply. The
///    updates themselves are _not_ applied in this phase.
/// 2. `update`: this is a phase where agents consider queued updates and compute a new state
///    accordingly.
pub trait Simulation: Send + Sync + Clone {
    type State: State;
    type World: State;
    type Update: Update;

    /// Computes updates for the specified agents and/or other agents.
    fn decide<P: Population<Self::State>>(&self,
                                          agent: Agent<Self::State>,
                                          world: Self::World,
                                          population: &P)
                                          -> Vec<(Uuid, Self::Update)>;

    /// Compute a final updated state given a starting state and updates.
    fn update(&self, state: Self::State, mut updates: Vec<Self::Update>) -> Self::State;
}

pub trait Population<S: State> {
    /// Create a new agent with the specified state, returning the new agent's id.
    fn spawn(&mut self, state: S) -> Uuid;

    /// Get an agent by id.
    fn get(&self, id: Uuid) -> Option<Agent<S>>;

    /// Deletes an agent by id.
    fn kill(&mut self, id: Uuid) -> ();
}

pub trait Manager<S: Simulation>: 'static {
    fn decide(&mut self) -> ();
    fn update(&mut self) -> ();
}
