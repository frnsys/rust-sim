use uuid::Uuid;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use futures::{future, collect, Future};
use futures_cpupool::{CpuPool, CpuFuture};
use population::SharedPopulation;
use simulation::Simulation;

pub trait Manager<S: Simulation>: 'static {
    fn decide(&mut self) -> ();
    fn update(&mut self) -> ();
}

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
