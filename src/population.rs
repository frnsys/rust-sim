use uuid::Uuid;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::collections::HashMap;
use std::collections::hash_map::Values;
use simulation::{Agent, State};

pub trait Population<S: State> {
    fn spawn(&mut self, state: S) -> Uuid;
    fn get(&self, id: Uuid) -> Option<Agent<S>>;
}

pub struct LocalPopulation<S: State> {
    pub agents: HashMap<Uuid, Agent<S>>,
}

impl<S: State> LocalPopulation<S> {
    pub fn new() -> LocalPopulation<S> {
        LocalPopulation { agents: HashMap::<Uuid, Agent<S>>::new() }
    }
}

impl<S: State> Population<S> for LocalPopulation<S> {
    fn spawn(&mut self, state: S) -> Uuid {
        let id = Uuid::new_v4();
        let agent = Agent {
            state: state,
            id: id.clone(),
        };
        self.agents.insert(agent.id, agent);
        id
    }

    fn get(&self, id: Uuid) -> Option<Agent<S>> {
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

#[derive(Clone)]
pub struct SharedPopulation<S: State> {
    pub population: Arc<RwLock<LocalPopulation<S>>>,
}

impl<S: State> SharedPopulation<S> {
    pub fn new() -> SharedPopulation<S> {
        SharedPopulation { population: Arc::new(RwLock::new(LocalPopulation::new())) }
    }
}

impl<S: State> Population<S> for SharedPopulation<S> {
    fn spawn(&mut self, state: S) -> Uuid {
        let mut pop = self.population.write().unwrap();
        pop.spawn(state)
    }

    fn get(&self, id: Uuid) -> Option<Agent<S>> {
        let pop = self.population.read().unwrap();
        match pop.agents.get(&id) {
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
