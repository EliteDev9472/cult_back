use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;
use crate::models::WebhookPayload;
use actix::{Actor, StreamHandler, AsyncContext, Handler, Message};

// Define a custom message type for WebSocket text messages
#[derive(Message)]
#[rtype(result = "()")]
struct WsMessage(String);

// WebSocket actor
struct WebSocketSession {
    id: Uuid,
    pool: PgPool,
    event_receiver: broadcast::Receiver<WebhookPayload>,
}

impl WebSocketSession {
    fn new(pool: PgPool, event_receiver: broadcast::Receiver<WebhookPayload>) -> Self {
        Self {
            id: Uuid::new_v4(),
            pool,
            event_receiver,
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Start a task to listen for events and send them to the WebSocket client
        let mut rx = self.event_receiver.resubscribe();
        let addr = ctx.address();

        actix::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&event) {
                    addr.do_send(WsMessage(json));
                }
            }
        });
    }
}

impl Handler<WsMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

// WebSocket endpoint handler
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    pool: web::Data<PgPool>,
    event_sender: web::Data<broadcast::Sender<WebhookPayload>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WebSocketSession::new(pool.get_ref().clone(), event_sender.subscribe()),
        &req,
        stream,
    )
}