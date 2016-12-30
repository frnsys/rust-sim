use uuid::Uuid;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use futures::{future, collect, Future};
use futures_cpupool::{CpuPool, CpuFuture};
use simulation::{Simulation, Agent, Population, Manager, State};

pub struct LocalManager<S: Simulation> {
    updates: HashMap<Uuid, Vec<S::Update>>,
    world: S::World,
    population: SharedPopulation<S::State>,
    simulation: S,
}

impl<S: Simulation + 'static> Manager<S> for LocalManager<S> {
    /// Calls the `decide` method on all agents.
    fn decide(&mut self) {
        let mut futs = Vec::new();
        let pool = CpuPool::new_num_cpus();
        let world = self.world.clone();
        let pop = self.population.population.read().unwrap();
        for agent in pop.agents.values() {
            let agent = agent.clone();
            let world = world.clone();
            let pop = self.population.clone();
            let sim = self.simulation.clone();
            let f: CpuFuture<Vec<(Uuid, S::Update)>, ()> =
                pool.spawn(future::lazy(move || {
                    future::finished(sim.decide(agent, world, &pop))
                }));
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
        let mut pop = self.population.population.write().unwrap();
        for (id, updates) in self.updates.drain() {
            match pop.agents.get_mut(&id) {
                Some(agent) => {
                    let state = self.simulation.update(agent.state.clone(), updates);
                    agent.state = state;
                }
                None => println!("No agent with id {}", id), // TODO this should probably log an error
            }
        }
        self.updates.clear();
    }
}

impl<S: Simulation> Manager<S> {
    fn new(simulation: S, world: S::World) -> LocalManager<S> {
        LocalManager {
            updates: HashMap::<Uuid, Vec<S::Update>>::new(),
            population: SharedPopulation::new(),
            simulation: simulation,
            world: world,
        }
    }
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

    fn kill(&mut self, id: Uuid) -> () {
        self.agents.remove(&id);
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

    fn kill(&mut self, id: Uuid) -> () {
        let mut pop = self.population.write().unwrap();
        pop.kill(id);
    }
}
