use std::thread;
use std::io::Error;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio_proto::TcpClient;
use tokio_core::io::Io;
use tokio_service::Service;
use tokio_core::reactor::Core;
use tokio_core::net::TcpListener;
use futures::{Future, Stream, Sink};
use super::proto::{Message, MsgPackProtocol, MsgPackCodec};

pub struct Router {
    addr: String,
}

impl Router {
    pub fn new(addr: String) -> Router {
        Router { addr: addr }
    }

    pub fn serve<F: Send + 'static, Req: Message + 'static, Res: Message + 'static>
        (&self,
         handle_req: F)
         -> thread::JoinHandle<()>
        where F: Fn(Req) -> Result<Res, Error>
    {
        let addr = self.addr.clone();
        thread::spawn(move || {
            let mut core = Core::new().unwrap();
            let handle = core.handle();
            let addr = addr.parse().unwrap();
            let tcp_socket = TcpListener::bind(&addr, &handle).unwrap();
            println!("Listening on: {}", addr);

            let f = Arc::new(handle_req); // TODO is this the best way to re-use the `handle_req` closure?
            let done = tcp_socket.incoming()
                .for_each(move |(socket, client_addr)| {
                    println!("Received connection from: {}", client_addr);
                    let (sink, stream) = socket.framed(MsgPackCodec::<Req, Res>::new())
                        .split();
                    let f = f.clone();
                    let conn = stream.forward(sink.with(move |req| {
                            let req: Req = req;
                            println!("{:?}", req);
                            f(req)
                        }))
                        .then(|_| Ok(()));
                    handle.spawn(conn);
                    Ok(())
                });
            let _ = core.run(done);
        })
    }

    fn request<Req: Message + 'static, Res: Message + 'static>(&self,
                                                               addr: SocketAddr,
                                                               req: Req)
                                                               -> Result<Res, Error> {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        println!("connecting to {}", addr);
        let proto: MsgPackProtocol<Res, Req> = MsgPackProtocol::new();
        let client = TcpClient::new(proto).connect(&addr, &handle);
        let res = core.run(client.and_then(|client| client.call(req)));
        res
    }
}
