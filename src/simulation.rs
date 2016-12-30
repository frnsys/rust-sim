use uuid::Uuid;
use std::fmt::Debug;
use population::Population;
use rustc_serialize::{Decodable, Encodable};

pub trait State: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}
impl<T> State for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait Update
    : Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {
}
impl<T> Update for T where T: Decodable + Encodable + Debug + Send + Sync + Clone + PartialEq {}

pub trait World: Decodable + Encodable + Debug + Send + Sync + Clone {}
impl<T> World for T where T: Decodable + Encodable + Debug + Send + Sync + Clone {}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub struct Agent<S: State> {
    pub id: Uuid,
    pub state: S,
}

pub trait Simulation: Send + Sync + Clone {
    type State: State;
    type Update: Update;
    type World: World;

    fn apply_update(&self, state: Self::State, update: Self::Update) -> Self::State;
    fn decide<P: Population<Self::State>>(&self,
                                          agent: Agent<Self::State>,
                                          world: Self::World,
                                          population: &P)
                                          -> Vec<(Uuid, Self::Update)>;
    fn update(&self, state: Self::State, mut updates: Vec<Self::Update>) -> Self::State {
        let mut state = state.clone();
        for update in updates.drain(..) {
            state = self.apply_update(state, update.clone());
        }
        state
    }
}
