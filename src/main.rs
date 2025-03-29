use actix_web::{
    web, App, HttpResponse, HttpServer, Responder, 
    middleware::{Logger, NormalizePath}, 
    guard, dev::ServiceRequest, error::ErrorUnauthorized
};

// use futures::future::{ok, Either};
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use dotenv::dotenv;
use std::{env, sync::Arc, time::Duration};
use serde::{Serialize};
use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

// Async job queue for processing events
use tokio::sync::mpsc;
use tokio::task;
use tokio::sync::broadcast;

mod models;
mod routes;
mod handlers;
mod errors;
mod websocket;
use websocket::websocket_handler;
use routes::app_routes;

// Configuration struct for webhook settings
#[derive(Clone)]
struct WebhookConfig {
    secret_key: String,
    max_concurrent_jobs: usize,
    job_queue_buffer: usize,
}

// Webhook payload structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum WebhookEventType {
    CultTokenCreated,
    CultTokenBuy,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WebhookPayload {
    id: Option<Uuid>,
    event_type: WebhookEventType,
    data: serde_json::Value,
    timestamp: i64,
}

// Response for webhook receipt
#[derive(Serialize)]
struct WebhookResponse {
    status: String,
    event_id: Uuid,
}


// Event processing queue
struct EventProcessor {
    sender: mpsc::Sender<WebhookPayload>,
    pool: Pool<Postgres>,
}

impl EventProcessor {
    async fn start(
        mut receiver: mpsc::Receiver<WebhookPayload>, 
        pool: Pool<Postgres>, 
        max_concurrent: usize
    ) {
        // Semaphore to limit concurrent processing
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

        while let Some(payload) = receiver.recv().await {
            let pool_clone = pool.clone();
            let semaphore_clone = semaphore.clone();

            // Spawn a task with limited concurrency
            task::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();

                match payload.event_type {
                    WebhookEventType::CultTokenCreated => {
                        if let Ok(event) = serde_json::from_value::<handlers::CultTokenCreatedEvent>(payload.data) {
                            let _ = handlers::handle_cult_token_created(event, &pool_clone).await;
                        }
                    },
                    WebhookEventType::CultTokenBuy => {
                        if let Ok(event) = serde_json::from_value::<handlers::CultTokenBuyEvent>(payload.data) {
                            let _ = handlers::handle_cult_token_buy(event, &pool_clone).await;
                        }
                    },
                }
            });
        }
    }
}

// Advanced authentication middleware
async fn webhook_authenticator(
    mut req: ServiceRequest, 
    credentials: web::Data<WebhookConfig>
) -> Result<ServiceRequest, actix_web::Error> {
    // Extract body first
    let body = req.extract::<bytes::Bytes>().await.map_err(ErrorUnauthorized)?;
    
    // Extract authentication headers
    let headers = req.headers();
    
    // Webhook signature verification
    let signature = headers.get("X-Webhook-Signature")
        .ok_or_else(|| ErrorUnauthorized("Missing signature"))?
        .to_str()
        .map_err(|_| ErrorUnauthorized("Invalid signature format"))?;
    
    // Timestamp replay protection
    let timestamp = headers.get("X-Webhook-Timestamp")
        .ok_or_else(|| ErrorUnauthorized("Missing timestamp"))?
        .to_str()
        .map_err(|_| ErrorUnauthorized("Invalid timestamp format"))?
        .parse::<i64>()
        .map_err(|_| ErrorUnauthorized("Invalid timestamp"))?;
    
    // Check timestamp is not too old (e.g., 5 minutes)
    let current_time = chrono::Utc::now().timestamp();
    if (current_time - timestamp).abs() > 300 {
        return Err(ErrorUnauthorized("Expired timestamp"));
    }

    // Verify HMAC signature
    let secret = credentials.secret_key.clone();
    
    match verify_signature(&secret, &body, signature) {
        Ok(true) => Ok(req),
        _ => Err(ErrorUnauthorized("Invalid signature"))
    }
}

// Signature verification function
fn verify_signature(
    secret: &str, 
    payload: &bytes::Bytes, 
    expected_signature: &str
) -> Result<bool, anyhow::Error> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
    mac.update(payload);
    
    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());
    
    Ok(signature == expected_signature)
}


// Webhook handler with advanced processing
async fn webhook_handler(
    payload: web::Json<WebhookPayload>,
    event_processor: web::Data<mpsc::Sender<WebhookPayload>>,
    event_sender: web::Data<broadcast::Sender<WebhookPayload>>,
) -> impl Responder {
    // Generate unique event ID if not provided
    let event_id = payload.id.unwrap_or_else(Uuid::new_v4);
    let payload = payload.into_inner();

    // Broadcast to WebSocket clients
    let _ = event_sender.send(payload.clone());

    // Add event to processing queue
    match event_processor.send(payload).await {
        Ok(_) => HttpResponse::Accepted().json(WebhookResponse {
            status: "event_queued".to_string(),
            event_id,
        }),
        Err(_) => HttpResponse::ServiceUnavailable().json(WebhookResponse {
            status: "queue_full".to_string(),
            event_id,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    env_logger::init();

    // Configuration from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let server_address = env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let webhook_secret = env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET must be set");

    // Database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await?;

    // Webhook configuration
    let webhook_config = WebhookConfig {
        secret_key: webhook_secret,
        max_concurrent_jobs: 100,
        job_queue_buffer: 10_000,
    };

    // Create event processing channel
    let (sender, receiver) = mpsc::channel(webhook_config.job_queue_buffer);

    // Start background event processor
    tokio::spawn(EventProcessor::start(
        receiver, 
        pool.clone(), 
        webhook_config.max_concurrent_jobs
    ));

        // Create broadcast channel for WebSocket events
        let (event_sender, _) = broadcast::channel::<WebhookPayload>(100);

        // Clone the sender for the webhook handler
        let webhook_event_sender = event_sender.clone();
    
    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(webhook_config.clone()))
            .app_data(web::Data::new(sender.clone()))
            .service(
                web::resource("/webhook")
                    .route(
                        web::post()
                            .guard(guard::Header("content-type", "application/json"))
                            .to(webhook_handler)
                    )
            )            .service(
                web::resource("/ws").route(web::get().to(websocket_handler))
            )
        .service(app_routes(web::Data::new(pool.clone())))
    })
    .bind(server_address)?
    .workers(4)  // Adjust based on CPU cores
    .run()
    .await?;

    Ok(())
}