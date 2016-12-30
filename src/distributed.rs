use std::io;
use std::ops::Deref;
use uuid::Uuid;
use rmp_serialize::decode::Error;
use rmp_serialize::{Encoder, Decoder};
use rustc_serialize::{Encodable, Decodable};
use redis::{Commands, Connection, ConnectionLike, Client, PipelineCommands, pipe};
use std::marker::PhantomData;
use simulation::{Agent, State, Simulation, Population, Manager};

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

fn get_agent<S: State>(id: Uuid, conn: &Connection) -> Option<Agent<S>> {
    let data = conn.get(id.to_string()).unwrap();
    Some(Agent {
        id: id,
        state: decode(data).unwrap(),
    })
}

fn set_agent<S: State>(id: Uuid, state: S, conn: &Connection) {
    // let data = json::encode(&state).unwrap();
    let data = encode(&state).unwrap();
    let _: () = conn.set(id.to_string(), data).unwrap();
}

pub struct DistPopulation<S: State> {
    conn: Connection,
    state: PhantomData<S>,
}

impl<S: State> DistPopulation<S> {
    pub fn new(addr: &str) -> DistPopulation<S> {
        let client = Client::open(addr).unwrap();
        DistPopulation {
            conn: client.get_connection().unwrap(),
            state: PhantomData,
        }
    }
}

impl<S: State> Population<S> for DistPopulation<S> {
    fn spawn(&mut self, state: S) -> Uuid {
        let id = Uuid::new_v4();
        set_agent(id, state, &self.conn);
        let _: () = self.conn.sadd("population", id.to_string()).unwrap();
        let _: () = self.conn.sadd("to_update", id.to_string()).unwrap();
        id
    }

    fn get(&self, id: Uuid) -> Option<Agent<S>> {
        get_agent(id, &self.conn)
    }

    fn kill(&mut self, id: Uuid) {
        let _: () = self.conn.del(id.to_string()).unwrap();
        let _: () = self.conn.srem("population", id.to_string()).unwrap();
    }
}

pub struct DistManager<S: Simulation> {
    conn: Connection,
    pub population: DistPopulation<S::State>,
}

impl<S: Simulation + 'static> Manager<S> for DistManager<S> {
    fn decide(&mut self) -> () {
        // TODO create tasks for each agent
        // actually...in the current implementation this doesn't really have to do anything?
    }
    fn update(&mut self) -> () {
        // TODO create tasks for each agent
        // actually...in the current implementation this doesn't really have to do anything?
    }
}

impl<S: Simulation> DistManager<S> {
    pub fn new(addr: &str, world: S::World) -> DistManager<S> {
        let client = Client::open(addr).unwrap();
        let conn = client.get_connection().unwrap();

        let data = encode(&world).unwrap();
        let _: () = conn.set("world", data).unwrap();

        DistManager {
            conn: conn,
            population: DistPopulation::new(addr),
        }
    }

    pub fn world(&self) -> S::World {
        let data = self.conn.get("world").unwrap();
        decode(data).unwrap()
    }
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
        let client = Client::open(self.addr.deref()).unwrap();
        let conn = client.get_connection().unwrap();
        loop {
            self.decide(&simulation, &conn);

            // check that all agents are ready to update
            while conn.scard::<&str, usize>("to_update").unwrap() !=
                  conn.scard::<&str, usize>("population").unwrap() {
            }

            self.update(&simulation, &conn);

            // check that all agents are ready to decide
            while conn.scard::<&str, usize>("to_decide").unwrap() !=
                  conn.scard::<&str, usize>("population").unwrap() {
            }
        }
    }

    fn decide(&self, simulation: &S, conn: &Connection) {
        let world: S::World = {
            let world_data = conn.get("world").unwrap();
            decode(world_data).unwrap()
        };
        while let Ok(id) = conn.spop::<&str, String>("to_decide") {
            let id = Uuid::parse_str(&id).unwrap();
            match get_agent::<S::State>(id, &conn) {
                Some(agent) => {
                    let updates = simulation.decide(agent, world.clone(), &self.population);
                    let mut rpipe = pipe();
                    for (id, update) in updates {
                        let data = encode(&update).unwrap();
                        let rpipe = rpipe.lpush(format!("updates:{}", id), data);
                    }
                    let rpipe = rpipe.sadd("to_update", id.to_string());
                    let _: () = rpipe.query(conn).unwrap();
                }
                None => (),
            }
        }
    }

    fn update(&self, simulation: &S, conn: &Connection) {
        while let Ok(id) = conn.spop::<&str, String>("to_update") {
            let updates = {
                let updates_data = conn.lrange(format!("updates:{}", id), 0, -1).unwrap();
                decode(updates_data).unwrap()
            };
            let id = Uuid::parse_str(&id).unwrap();
            match get_agent::<S::State>(id, &conn) {
                Some(agent) => {
                    let new_state = simulation.update(agent.state.clone(), updates);
                    set_agent(id, new_state, &conn);
                    let _: () = conn.sadd("to_decide", id.to_string()).unwrap();
                }
                None => (),
            }
        }
    }
}
