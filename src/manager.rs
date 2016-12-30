use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use futures::{future, collect, Future};
use futures_cpupool::{CpuPool, CpuFuture};
use population::{SharedPopulation, LocalPopulation};
use simulation::Simulation;

pub trait Manager<S: Simulation>: Send + Sync + 'static {
    fn new(world: S::World) -> Self;
    fn decide(&mut self) -> ();
    fn update(&mut self) -> ();
}

pub struct LocalManager<S: Simulation> {
    updates: HashMap<usize, Vec<S::Update>>,
    world: S::World,
    pub population: SharedPopulation<LocalPopulation<S::State>>,
    simulation: S,
}

impl<S: Simulation + 'static> Manager<S> for LocalManager<S> {
    fn new(world: S::World) -> LocalManager<S> {
        LocalManager {
            updates: HashMap::<usize, Vec<S::Update>>::new(),
            world: world.clone(),
            population: Arc::new(RwLock::new(LocalPopulation::new())),
            simulation: S::new(world),
        }
    }

    /// Calls the `decide` method on all agents.
    fn decide(&mut self) {
        let mut futs = Vec::new();
        let pool = CpuPool::new_num_cpus();
        let world = self.world.clone();
        let pop = self.population.read().unwrap();
        for agent in pop.agents.values() {
            let pop = self.population.clone();
            let agent = agent.clone();
            let world = world.clone();
            let sim = self.simulation.clone();
            let f: CpuFuture<Vec<(usize, S::Update)>, ()> =
                pool.spawn(future::lazy(move || future::finished(sim.decide(agent, world, pop))));
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
        let mut population = self.population.write().unwrap();
        for (id, updates) in self.updates.drain() {
            match population.agents.get_mut(&id) {
                Some(agent) => self.simulation.update(agent, updates),
                None => println!("No agent with id {}", id), // TODO this should probably log an error
            }
        }
        self.updates.clear();
    }
}
