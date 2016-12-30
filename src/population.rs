use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use simulation::{Agent, State};

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
