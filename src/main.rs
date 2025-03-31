use actix_service::Service;
use actix_web::{
    dev::ServiceRequest,
    error::ErrorUnauthorized,
    guard,
    middleware::{Logger, NormalizePath},
    web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use dotenv::dotenv;
use futures::future::{ok, Either};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{env, sync::Arc, time::Duration};
use uuid::Uuid;

// Async job queue for processing events
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task;

mod auth;
mod community_airdrops;
mod diamond_hands;
mod errors;
mod handlers;
mod models;
mod routes;
mod websocket;
use auth::middleware::ApiGuard;
use community_airdrops::community_airdrops::update_all_communities;
use diamond_hands::diamond_hands::{
    get_eligible_accounts, select_weighted_sample, update_diamond_hands, SAMPLE_SIZE,
};
use websocket::websocket_handler;

// Advanced authentication middleware
// async fn webhook_authenticator(
//     req: ServiceRequest,
//     credentials: web::Data<models::WebhookConfig>,
// ) -> Result<ServiceRequest, actix_web::Error> {
//     // Extract body first
//     let body = req
//         .extract::<bytes::Bytes>()
//         .await
//         .map_err(ErrorUnauthorized)?;

//     // Extract authentication headers
//     let headers = req.headers();

//     // Webhook signature verification
//     let signature = headers
//         .get("X-Webhook-Signature")
//         .ok_or_else(|| ErrorUnauthorized("Missing signature"))?
//         .to_str()
//         .map_err(|_| ErrorUnauthorized("Invalid signature format"))?;

//     // Timestamp replay protection
//     let timestamp = headers
//         .get("X-Webhook-Timestamp")
//         .ok_or_else(|| ErrorUnauthorized("Missing timestamp"))?
//         .to_str()
//         .map_err(|_| ErrorUnauthorized("Invalid timestamp format"))?
//         .parse::<i64>()
//         .map_err(|_| ErrorUnauthorized("Invalid timestamp"))?;

//     // Check timestamp is not too old (e.g., 5 minutes)
//     let current_time = chrono::Utc::now().timestamp();
//     if (current_time - timestamp).abs() > 300 {
//         return Err(ErrorUnauthorized("Expired timestamp"));
//     }

//     // Verify HMAC signature
//     let secret = credentials.secret_key.clone();

//     match verify_signature(&secret, &body, signature) {
//         Ok(true) => Ok(req),
//         _ => Err(ErrorUnauthorized("Invalid signature")),
//     }
// }

// // Signature verification function
// fn verify_signature(
//     secret: &str,
//     payload: &bytes::Bytes,
//     expected_signature: &str,
// ) -> Result<bool, anyhow::Error> {
//     let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
//     mac.update(payload);

//     let result = mac.finalize();
//     let signature = hex::encode(result.into_bytes());

//     Ok(signature == expected_signature)
// }

// Event processing queue
struct EventProcessor {
    sender: mpsc::Sender<models::WebhookPayload>,
    pool: Pool<Postgres>,
}

impl EventProcessor {
    async fn start(
        mut receiver: mpsc::Receiver<models::WebhookPayload>,
        pool: Pool<Postgres>,
        max_concurrent: usize,
    ) {
        
        // Semaphore to limit concurrent processing
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

        while let Some(payload) = receiver.recv().await {
            let pool_clone = pool.clone();
            let semaphore_clone = semaphore.clone();
            

            // Spawn a task with limited concurrency
            task::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();

                // Start a transaction
                let mut tx = match pool_clone.begin().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        log::error!("Failed to start transaction: {}", e);
                        return;
                    }
                };
                let result = match payload.event_type {
                    models::WebhookEventType::CultTokenCreated => {
                        println!("Processing CultTokenCreated event");
                        match serde_json::from_value::<handlers::CultTokenCreatedEvent>(
                            payload.data,
                        ) {
                            Ok(event) => {
                                // Pass mutable borrow of tx, handler returns Result
                                handlers::handle_cult_token_created(event, &mut tx).await
                            }
                            Err(e) => {
                                println!("Failed to deserialize CultTokenCreatedEvent: {}", e);
                                // Treat deserialization error as a processing failure
                                Err(anyhow::anyhow!("Deserialization failed: {}", e))
                            }
                        }
                    }
                    models::WebhookEventType::CultTokenBuy => {
                        println!("Processing CultTokenBuy event");
                        match serde_json::from_value::<handlers::CultTokenBuyEvent>(
                            payload.data,
                        ) {
                            Ok(event) => {
                                // Pass mutable borrow of tx, handler returns Result
                                handlers::handle_cult_token_buy(event, &mut tx).await
                            }
                            Err(e) => {
                                println!("Failed to deserialize CultTokenBuyEvent: {}", e);
                                // Treat deserialization error as a processing failure
                                Err(anyhow::anyhow!("Deserialization failed: {}", e))
                            }
                        }
                    }
                };

                // Commit or rollback the transaction
                match result {
                    Ok(_) => {
                        // If result is Ok, attempt to commit
                        if let Err(e) = tx.commit().await {
                            // Log commit error, rollback is not possible anymore
                            log::error!("Failed to commit transaction after successful processing: {}", e);
                        } else {
                            log::info!("Successfully processed event and committed transaction.");
                        }
                    }
                    Err(e) => {
                        // If result was an error (either from handler or deserialization), rollback
                        log::error!("Event processing failed: {}. Rolling back.", e);
                        if let Err(rollback_err) = tx.rollback().await {
                            log::error!("Failed to rollback transaction: {}", rollback_err);
                        }
                    }
                }
            });
        }
    }
}

// Webhook handler with advanced processing
async fn webhook_handler(
    payload: web::Json<models::WebhookPayload>,
    event_processor: web::Data<mpsc::Sender<models::WebhookPayload>>,
    event_sender: web::Data<broadcast::Sender<models::WebhookPayload>>,
) -> impl Responder {
    println!("RECEIVED WEBHOOK EVENT");
    // Generate unique event ID if not provided
    let event_id = payload.id.clone();
    let payload = payload.into_inner();

    // Broadcast to WebSocket clients
//    let _ = event_sender.send(payload.clone());
    

    // Add event to processing queue
    match event_processor.send(payload).await {
        Ok(_) => HttpResponse::Accepted().json(models::WebhookResponse {
            status: "event_queued".to_string(),
            event_id,
        }),
        Err(_) => HttpResponse::ServiceUnavailable().json(models::WebhookResponse {
            status: "queue_full".to_string(),
            event_id,
        }),
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();
    env_logger::init();

    // Configuration from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let server_address =
        env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let webhook_secret = env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET must be set");

    // Database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Webhook configuration
    let webhook_config = models::WebhookConfig {
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
        webhook_config.max_concurrent_jobs,
    ));

    // Create broadcast channel for WebSocket events
    let (event_sender, _) = broadcast::channel::<models::WebhookPayload>(100);

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
            .app_data(web::Data::new(webhook_event_sender.clone()))  
            .service(
                web::scope("/api")
                    //.wrap(ApiGuard::new())
                    // New API routes
                    // .service(web::resource("/tokens").route(web::get().to(routes::get_tokens)))
                    // .service(web::resource("/tokens/{token_id}").route(web::get().to(routes::get_token)))
                    // .service(web::resource("/tokens/{token_id}/trades").route(web::get().to(routes::get_token_trades)))
                    .service(web::resource("/profile/{user_id}").route(web::get().to(routes::get_profile)))
                    .service(web::resource("/profileData/{user_id}").route(web::get().to(routes::get_profile_data)))
                  //  .service(web::resource("/profile/{user_id}").route(web::put().to(routes::update_profile)))
                    .service(web::resource("/diamond_hands").route(web::get().to(routes::get_diamond_hands)))
                    .service(web::resource("/cult_tokens/{offset}/{limit}").route(web::get().to(routes::get_cult_tokens)))
                    .service(web::resource("/top_coins").route(web::get().to(routes::get_top_coins)))
                    .service(web::resource("/cult_token/{token_address}").route(web::get().to(routes::get_token_data)))
                    .service(web::resource("/top_holders/{token_address}/{offset}/{limit}").route(web::get().to(routes::get_top_holders)))
                    .service(web::resource("/trades/{token_address}/{offset}/{limit}").route(web::get().to(routes::get_token_trades)))
                    .service(web::resource("/communities").route(web::get().to(routes::get_all_communities))),
            )
            .service(
                web::scope("/admin")
                    .wrap(ApiGuard::new())
                    .service(
                        web::resource("/run-diamond-hands")
                            .route(web::post().to(diamond_hands_handler)),
                    )
                    .service(
                        web::resource("/run-community-airdrops")
                            .route(web::post().to(community_airdrops_handler)),
                    ),
            )
            .service(
                web::resource("/webhook").route(
                    web::post()
                        .guard(guard::Header("content-type", "application/json"))
                       // .wrap(webhook_authenticator)
                        .to(webhook_handler),
                ),
            )
            .service(web::resource("/ws").route(web::get().to(websocket_handler)))
    })
    .bind(server_address)?
    .workers(4)
    .run()
    .await
}


async fn community_airdrops_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let result: Result<_, Box<dyn std::error::Error>> = async {
        let api_key = env::var("ALCHEMY_API_KEY")
            .map_err(|_| "Failed to retrieve Alchemy API key from environment")?;

        if api_key.trim().is_empty() {
            return Err("Alchemy API key is empty".into());
        }

        let update_result = update_all_communities(&pool, &api_key)
            .await
            .map_err(|e| format!("Community update failed: {}", e))?;

        Ok(())
    }
    .await;

    // Comprehensive error handling and response generation
    match result {
        Ok(updated_count) => Ok(HttpResponse::Ok().json(json!({
            "message": "Community List Updated Successfully",
            "communities_updated": updated_count
        }))),
        Err(e) => {
            // Log the error for internal tracking
            log::error!("Community airdrops handler error: {}", e);

            // Differentiate error responses
            if e.to_string().contains("API key") {
                Ok(HttpResponse::Unauthorized().body("Authentication failed: Invalid API key"))
            } else if e.to_string().contains("network") {
                Ok(HttpResponse::ServiceUnavailable().body("Network connectivity issue"))
            } else if e.to_string().contains("database") {
                Ok(HttpResponse::InternalServerError().body("Database operation failed"))
            } else {
                Ok(HttpResponse::InternalServerError().body(format!("Execution failed: {}", e)))
            }
        }
    }
}

// Handler functions extracted for better organization
async fn diamond_hands_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let result: Result<_, Box<dyn std::error::Error>> = async {
        let mut tx = pool.begin().await?;
        let accounts = get_eligible_accounts(&mut tx).await?;
        let selected = select_weighted_sample(accounts, SAMPLE_SIZE);
        update_diamond_hands(&mut tx, selected).await?;
        tx.commit().await?;
        Ok(())
    }
    .await;

    // Match and handle different error scenarios
    match result {
        Ok(_) => Ok(HttpResponse::Ok().body("Diamond hands executed successfully")),
        Err(e) => {
            log::error!("Diamond hands execution failed: {}", e);
            if e.to_string().contains("database connection") {
                Ok(HttpResponse::ServiceUnavailable().body("Database connection error"))
            } else if e.to_string().contains("No eligible accounts") {
                Ok(HttpResponse::BadRequest().body("No eligible accounts found"))
            } else {
                Ok(HttpResponse::InternalServerError().body(format!("Execution failed: {}", e)))
            }
        }
    }
}

