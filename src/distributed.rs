use redis;
use std::io;
use std::ops::Deref;
use redis::Commands;
use uuid::Uuid;
use rmp_serialize::decode::Error;
use rmp_serialize::{Encoder, Decoder};
use rustc_serialize::{Encodable, Decodable};
use std::marker::PhantomData;
use simulation::{Agent, State, Simulation};
use population::Population;
use manager::Manager;
use oppgave::Queue;

fn decode<R: Decodable>(inp: String) -> Result<R, Error> {
    let mut decoder = Decoder::new(inp.as_bytes());
    Decodable::decode(&mut decoder)
}

fn encode<R: Encodable>(data: R) -> Result<String, io::Error> {
    let mut buf = Vec::<u8>::new();
    match data.encode(&mut Encoder::new(&mut buf)) {
        Ok(_) => {
            match String::from_utf8(buf) {
                Ok(v) => Ok(v),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, format!("{}", e))),
            }
        }
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, format!("{}", e))),
    }
}

pub struct DistPopulation<S: State> {
    conn: redis::Connection,
    state: PhantomData<S>,
}

impl<S: State> DistPopulation<S> {
    pub fn new(addr: &str) -> DistPopulation<S> {
        let client = redis::Client::open(addr).unwrap();
        DistPopulation {
            conn: client.get_connection().unwrap(),
            state: PhantomData,
        }
    }
}

impl<S: State> Population<S> for DistPopulation<S> {
    fn spawn(&mut self, state: S) -> Uuid {
        let id = Uuid::new_v4();
        // let data = json::encode(&state).unwrap();
        let data = encode(&state).unwrap();

        // Add agent-state to redis
        let _: () = self.conn.set(id.to_string(), data).unwrap();
        id
    }

    fn get(&self, id: Uuid) -> Option<Agent<S>> {
        let data = self.conn.get(id.to_string()).unwrap();
        Some(Agent {
            id: id,
            state: decode(data).unwrap(),
        })
    }
}

pub struct DistManager<S: Simulation> {
    conn: redis::Connection,
    simulation: PhantomData<S>,
}

impl<S: Simulation + 'static> Manager<S> for DistManager<S> {
    fn decide(&mut self) -> () {
        // TODO create tasks for each agent
    }
    fn update(&mut self) -> () {
        // TODO create tasks for each agent
    }
}

impl<S: Simulation> DistManager<S> {
    fn new(addr: &str, world: S::World) -> DistManager<S> {
        let client = redis::Client::open(addr).unwrap();
        // TODO where does the world go
        DistManager {
            conn: client.get_connection().unwrap(),
            simulation: PhantomData,
        }
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq, Clone)]
pub enum DistTask<S: State> {
    Decide(String, S),
    Update(String, S),
}

pub struct DistWorker<S: Simulation> {
    addr: String,
    population: DistPopulation<S::State>,
}

impl<S: Simulation> DistWorker<S> {
    pub fn new(addr: &str) -> DistWorker<S> {
        DistWorker {
            addr: addr.to_owned(),
            population: DistPopulation::new(addr),
        }
    }

    pub fn start(&self, simulation: S) {
        let client = redis::Client::open(self.addr.deref()).unwrap();
        let queue = {
            let conn = client.get_connection().unwrap();
            Queue::new("default".into(), conn)
        };
        let conn = client.get_connection().unwrap();
        // TODO need to check new world, not just get it once
        let world: S::World = {
            let world_data = conn.get("world").unwrap();
            decode(world_data).unwrap()
        };
        while let Some(task) = queue.next::<DistTask<S::State>>() {
            let task = task.unwrap();
            match task.inner() {
                &DistTask::Decide(ref id, ref state) => {
                    let agent = Agent {
                        id: Uuid::parse_str(&id).unwrap(),
                        state: state.clone(),
                    };
                    let updates = simulation.decide(agent, world.clone(), &self.population);
                    for update in updates {
                        let data = encode(&update).unwrap();
                        let _: () = conn.lpush(format!("updates:{}", id), data).unwrap();
                    }
                }
                &DistTask::Update(ref id, ref state) => {
                    let updates = {
                        let updates_data = conn.get(format!("updates:{}", id)).unwrap();
                        decode(updates_data).unwrap()
                    };
                    let new_state = simulation.update(state.clone(), updates);

                    let data = encode(&new_state).unwrap();
                    let _: () = conn.set(id, data).unwrap();
                }
            }
        }
    }
}
