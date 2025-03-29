use actix_web::{web, Scope};
use sqlx::PgPool;
use crate::handlers::*;

pub fn app_routes(pool: web::Data<PgPool>) -> Scope {
    web::scope("/api")
        .app_data(pool.clone())
        .route("/cult_tokens", web::get().to(get_cult_tokens))
        .route("/top_coins", web::get().to(get_top_coins))
        .route("/cult_token/{token_address}", web::get().to(get_token_data))
        .route("/top_holders/{token_address}", web::get().to(get_top_holders))
        .route("/trades/{token_address}", web::get().to(get_token_trades))
        .route("/tokens_created/{account_id}", web::get().to(get_tokens_created))
        .route("/account/{account_id}", web::get().to(get_account_details))
}


