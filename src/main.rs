//! Simple echo websocket server.
//! Open `http://localhost:8080/ws/index.html` in browser
//! or [python console client](https://github.com/actix/examples/blob/master/websocket/websocket-client.py)
//! could be used for testing.

use actix::prelude::*;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

/// do websocket handshake and start `MyWebSocket` actor
fn ws_index(r: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(MyWebSocket::new(), &r, stream)
}

/// websocket connection is long running connection, it easier
/// to handle with an actor
struct MyWebSocket {
}

impl Actor for MyWebSocket {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for `ws::Message`
impl StreamHandler<ws::Message, ws::ProtocolError> for MyWebSocket {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        // process websocket messages
        match msg {
            ws::Message::Ping(msg) => {
                ctx.pong(msg);
            }
            ws::Message::Pong(msg) => {
            }
            ws::Message::Text(text) => {
                ctx.text(text)
            }
            ws::Message::Binary(bin) => {
                ctx.binary(bin)
            }
            ws::Message::Close(_msg) => {
                ctx.stop();
            }
            ws::Message::Nop => {
            }
        }
    }
}

impl MyWebSocket {
    fn new() -> Self {
        Self { }
    }
}

fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            // websocket route
            .service(web::resource("/").route(web::get().to(ws_index)))
    })
    // start http server on 127.0.0.1:9001
    .bind("0.0.0.0:9001")?
    .run()
}
