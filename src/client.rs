use std::time::Duration;
//use std::{io, thread};

use actix::io::SinkWrite;
use actix::*;
use actix_codec::{AsyncRead, AsyncWrite, Framed};
//use actix_web::http::{header, Method};
use awc::{
    error::WsProtocolError,
    ws::{CloseCode, CloseReason, Codec, Frame, Message},
    Client,
};
use bytes::Bytes;
use futures::{
    lazy,
    stream::{SplitSink, Stream},
    Future,
};
use std::io;
use std::io::Write;

fn main() {
    ::std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let sys = actix::System::new("ws-example");

    Arbiter::spawn(lazy(|| {
        Client::new()
            .ws("ws://127.0.0.1:9001/getCaseCount")
            .connect()
            .map_err(|e| {
                println!("Error: {:?}", e);
            })
            .map(|(_response, framed)| {
                let (sink, stream) = framed.split();
                AutobahnCtl::create(|ctx| {
                    AutobahnCtl::add_stream(stream, ctx);
                    AutobahnCtl(SinkWrite::new(sink, ctx))
                });
            })
    }));

    let _ = sys.run();
}

struct AutobahnCtl<T>(SinkWrite<SplitSink<Framed<T, Codec>>>)
where
    T: AsyncRead + AsyncWrite;

#[derive(Message)]
struct ClientCommand(String);

impl<T: 'static> Actor for AutobahnCtl<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // start heartbeats otherwise server will disconnect after 10 seconds
        self.hb(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        // Stop application on disconnect
        //System::current().stop();
    }
}

impl<T: 'static> AutobahnCtl<T>
where
    T: AsyncRead + AsyncWrite,
{
    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::new(1, 0), |act, ctx| {
            act.0.write(Message::Ping(String::new())).unwrap();
            act.hb(ctx);

            // client should also check for a timeout here, similar to the
            // server code
        });
    }
}

/// Handle stdin commands
impl<T: 'static> Handler<ClientCommand> for AutobahnCtl<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Result = ();

    fn handle(&mut self, msg: ClientCommand, _ctx: &mut Context<Self>) {
        self.0.write(Message::Text(msg.0)).unwrap();
    }
}

/// Handle server websocket messages
impl<T: 'static> StreamHandler<Frame, WsProtocolError> for AutobahnCtl<T>
where
    T: AsyncRead + AsyncWrite,
{
    fn handle(&mut self, msg: Frame, _ctx: &mut Context<Self>) {
        if let Frame::Text(Some(txt)) = msg {
            if let Ok(txt) = String::from_utf8(txt.to_vec()) {
                if let Ok(count) = txt.as_str().parse::<u64>() {
                    print!("Will run {} cases...", count);
                    Arbiter::spawn(lazy(move || {
                        Client::new()
                            .ws("ws://127.0.0.1:9001/runCase?case=1&agent=awc")
                            .max_frame_size(65_536_000)
                            .connect()
                            .map_err(move |e| {
                                println!("Error runCase({}): {:?}", count, e);
                            })
                            .map(move |(_response, framed)| {
                                let (sink, stream) = framed.split();
                                AutobahnWorker::create(move |ctx| {
                                    AutobahnWorker::add_stream(stream, ctx);
                                    AutobahnWorker(SinkWrite::new(sink, ctx), 1, count)
                                });
                            })
                    }));
                }
            };
        }
    }

    fn started(&mut self, _ctx: &mut Context<Self>) {}

    fn finished(&mut self, ctx: &mut Context<Self>) {
        ctx.stop()
    }
}

impl<T: 'static> actix::io::WriteHandler<WsProtocolError> for AutobahnCtl<T> where
    T: AsyncRead + AsyncWrite
{
}

struct AutobahnWorker<T>(SinkWrite<SplitSink<Framed<T, Codec>>>, u64, u64)
where
    T: AsyncRead + AsyncWrite;

impl<T: 'static> AutobahnWorker<T>
where
    T: AsyncRead + AsyncWrite,
{
    fn next(&mut self) {
        print!(".");
        if self.1 == self.2 {
            println!(" done!");
            Arbiter::spawn(lazy(move || {
                Client::new()
                    .ws("ws://127.0.0.1:9001/updateReports?agent=awc")
                    .connect()
                    .map_err(|e| {
                        println!("Error updateReports: {:?}", e);
                    })
                    .map(move |(_response, framed)| {
                        let (sink, stream) = framed.split();
                        AutobahnFinish::create(move |ctx| {
                            AutobahnFinish::add_stream(stream, ctx);
                            AutobahnFinish(SinkWrite::new(sink, ctx))
                        });
                    })
            }));
        } else {
            let i = self.1 + 1;
            let count = self.2;
            Arbiter::spawn(lazy(move || {
                Client::new()
                    .ws(&format!("ws://127.0.0.1:9001/runCase?case={}&agent=awc", i))
                    .max_frame_size(65_536_000)
                    .connect()
                    .map_err(move |e| {
                        println!("Error runCase({}/{}): {:?}", i, count, e);
                    })
                    .map(move |(_response, framed)| {
                        let (sink, stream) = framed.split();
                        AutobahnWorker::create(move |ctx| {
                            AutobahnWorker::add_stream(stream, ctx);
                            AutobahnWorker(SinkWrite::new(sink, ctx), i, count)
                        });
                    })
            }));
        }
        io::stdout().flush().unwrap();
    }
}
impl<T: 'static> actix::io::WriteHandler<WsProtocolError> for AutobahnWorker<T> where
    T: AsyncRead + AsyncWrite
{
}

//impl<T: 'static> AutobahnWorker<T> where T: AsyncRead + AsyncWrite {}
impl<T: 'static> Actor for AutobahnWorker<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        //println!("Started worker {}", self.1);
        // start heartbeats otherwise server will disconnect after 10 seconds
        //self.hb(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        //println!("Stopped worker {}", self.1);

        // Stop application on disconnect
        //System::current().stop();
    }
}

/// Handle stdin commands
impl<T: 'static> Handler<ClientCommand> for AutobahnWorker<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Result = ();

    fn handle(&mut self, _msg: ClientCommand, _ctx: &mut Context<Self>) {
        //self.0.write(Message::Text(msg.0)).unwrap();
    }
}

/// Handle server websocket messages
impl<T: 'static> StreamHandler<Frame, WsProtocolError> for AutobahnWorker<T>
where
    T: AsyncRead + AsyncWrite,
{
    fn handle(&mut self, msg: Frame, ctx: &mut Context<Self>) {
        match msg {
            Frame::Close(_) => {
                self.next();
                self.0.write(Message::Close(None)).unwrap();
            }
            Frame::Ping(data) => {
                self.0.write(Message::Pong(data)).unwrap();
            }
            Frame::Text(Some(data)) => {
                if let Ok(txt) = String::from_utf8(data.to_vec()) {
                    self.0.write(Message::Text(txt)).unwrap();
                } else {
                    self.0
                        .write(Message::Close(Some(CloseReason {
                            code: CloseCode::Error,
                            description: None,
                        })))
                        .unwrap();
                    self.next();
                    ctx.stop();
                }
            }
            Frame::Text(None) => {
                self.0.write(Message::Text(String::new())).unwrap();
            }
            Frame::Binary(Some(data)) => {
                self.0.write(Message::Binary(data.into())).unwrap();
            }
            Frame::Binary(None) => {
                self.0.write(Message::Binary(Bytes::new())).unwrap();
            }
            Frame::Continue => {
                //self.0.write(Message::Ping(String::new())).unwrap();
                ()
            }
            Frame::Pong(_) => (),
        }
    }

    fn error(&mut self, error: WsProtocolError, _ctx: &mut Context<Self>) -> Running {
        match error {
            _ => self
                .0
                .write(Message::Close(Some(CloseReason {
                    code: CloseCode::Protocol,
                    description: None,
                })))
                .unwrap(),
        };
        self.next();
        Running::Stop
    }

    fn started(&mut self, _ctx: &mut Context<Self>) {
        //println!("Connected");
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        //println!("Server disconnected");
        self.0.write(Message::Close(None)).unwrap();
        ctx.stop()
    }
}

struct AutobahnFinish<T>(SinkWrite<SplitSink<Framed<T, Codec>>>)
where
    T: AsyncRead + AsyncWrite;

impl<T: 'static> Actor for AutobahnFinish<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {}

    fn stopped(&mut self, _: &mut Context<Self>) {
        System::current().stop();
    }
}

impl<T: 'static> AutobahnFinish<T> where T: AsyncRead + AsyncWrite {}

/// Handle stdin commands
impl<T: 'static> Handler<ClientCommand> for AutobahnFinish<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Result = ();

    fn handle(&mut self, msg: ClientCommand, _ctx: &mut Context<Self>) {
        self.0.write(Message::Text(msg.0)).unwrap();
    }
}

/// Handle server websocket messages
impl<T: 'static> StreamHandler<Frame, WsProtocolError> for AutobahnFinish<T>
where
    T: AsyncRead + AsyncWrite,
{
    fn handle(&mut self, msg: Frame, _ctx: &mut Context<Self>) {
        match msg {
            Frame::Close(_) => {
                self.0.write(Message::Close(None)).unwrap();
                println!("Reports updated.");
                println!("Test suite finished!");
            }
            Frame::Ping(data) => {
                self.0.write(Message::Pong(data)).unwrap();
            }
            Frame::Text(Some(data)) => {
                self.0
                    .write(Message::Text(String::from_utf8_lossy(&data).into()))
                    .unwrap();
            }
            Frame::Binary(Some(data)) => {
                self.0.write(Message::Binary(data.into())).unwrap();
            }
            _ => (), //println!("AutobahnWorker: {:?}", msg),
        }
    }

    fn started(&mut self, _ctx: &mut Context<Self>) {}

    fn finished(&mut self, ctx: &mut Context<Self>) {
        ctx.stop()
    }
}

impl<T: 'static> actix::io::WriteHandler<WsProtocolError> for AutobahnFinish<T> where
    T: AsyncRead + AsyncWrite
{
}
