// use anyhow::Ok;
// use chrono::{DateTime, TimeZone, Utc};
// use serde_json::Value;
use sqlx::{Pool, Postgres};
// use std::sync::Arc;
// use std::time::SystemTime;
// use reqwest;
use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use std::result::Result::Ok;

use crate::models::{Account, CultToken, TokenBalance, TokenTradeRow};

pub async fn handle_cult_token_created(
    event: CultTokenCreatedEvent,
    pool: &Pool<Postgres>,
) -> Result<(), anyhow::Error> {
    Ok(())
}

pub async fn handle_cult_token_buy(
    event: CultTokenBuyEvent,
    pool: &Pool<Postgres>,
) -> Result<(), anyhow::Error> {
    Ok(())
}
#[derive(serde::Deserialize)]
pub struct CultTokenCreatedEvent {
    pub token_address: String,
    pub token_creator: String,
    pub airdrop_contract: String,
    pub factory_address: String,
    pub protocol_fee_recipient: String,
    pub bonding_curve: String,
    pub token_uri: String,
    pub name: String,
    pub symbol: String,
    pub pool_address: String,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub block_hash: String,
}

#[derive(serde::Deserialize)]
pub struct CultTokenBuyEvent {
    pub srcAddress:String,
    pub buyer: String,
    pub recipient: String,
    pub order_referrer: String,
    pub total_eth: i64,
    pub eth_fee: i64,
    pub eth_sold: i64,
    pub tokens_bought: i64,
    pub buyer_token_balance: i64,
    pub total_supply: i64,
    pub market_type: u8,
    pub block_timestamp: u64,
    pub block_hash: String,
}

#[derive(serde::Deserialize)]
pub struct CultTokenSellEvent {
    pub seller: String,
    pub recipient: String,
    pub order_referrer: String,
    pub total_eth: i64,
    pub eth_fee: i64,
    pub eth_bought: i64,
    pub tokens_sold: i64,
    pub seller_token_balance: i64,
    pub total_supply: i64,
    pub market_type: u8,
    pub block_timestamp: u64,
    pub block_hash: String,
}

#[derive(serde::Deserialize)]
pub struct CultTokenTransferEvent {
    pub from: String,
    pub to: String,
    pub from_token_balance: i64,
    pub to_token_balance: i64,
    pub block_timestamp: u64,
    pub block_hash: String,
}

#[derive(serde::Deserialize)]
pub struct CultTokenFeesEvent {
    pub order_referrer: String,
    pub order_referrer_fee: i64,
    pub block_timestamp: u64,
    pub block_hash: String,
}


pub async fn get_cult_tokens(pool: web::Data<PgPool>) -> HttpResponse {
    let query = sqlx::query_as::<_, CultToken>(
        "SELECT * FROM public.cult_token ORDER BY block_timestamp DESC"
    );
    match query.fetch_all(pool.get_ref()).await {
        Ok(tokens) => HttpResponse::Ok().json(tokens),
        Err(err) => {
            eprintln!("Failed to fetch cult tokens: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch cult tokens")
        }
    }
}

pub async fn get_top_coins(pool: web::Data<PgPool>) -> HttpResponse {
    let query = sqlx::query_as::<_, CultToken>(
        "SELECT * FROM public.cult_token ORDER BY holder_count DESC LIMIT 10"
    );

    match query.fetch_all(pool.get_ref()).await {
        Ok(tokens) => HttpResponse::Ok().json(tokens),
        Err(err) => {
            eprintln!("Failed to fetch top coins: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch top coins")
        }
    }
}

pub async fn get_token_data(
    pool: web::Data<PgPool>,
    token_address: web::Path<String>,
) -> HttpResponse {
    let result = sqlx::query_as::<_, CultToken>(
        "SELECT * FROM public.cult_token WHERE id = $1"
    )
    .bind(token_address.into_inner())
    .fetch_optional(pool.get_ref())
    .await;

    match result {
        Ok(Some(token)) => HttpResponse::Ok().json(token),
        Ok(None) => HttpResponse::NotFound().body("Token not found"),
        Err(e) => {
            eprintln!("Error fetching token: {:?}", e);
            HttpResponse::InternalServerError().body("DB error")
        }
    }
}

pub async fn get_top_holders(
    pool: web::Data<PgPool>,
    token_address: web::Path<String>,
) -> HttpResponse {
    println!("---------------->");
    let result = sqlx::query_as::<_, TokenBalance>(
        "SELECT * FROM public.token_balance WHERE token_id = $1 ORDER BY value DESC LIMIT 100"
    )
    .bind(token_address.into_inner())
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(balances) => HttpResponse::Ok().json(balances),
        Err(e) => {
            eprintln!("Error fetching top holders: {:?}", e);
            HttpResponse::InternalServerError().body("DB error")
        }
    }
}

pub async fn get_token_trades(
    pool: web::Data<PgPool>,
    token_address: web::Path<String>,
) -> HttpResponse {
    let result = sqlx::query_as::<_, TokenTradeRow>(
        "SELECT * FROM public.token_trade WHERE token_id = $1 ORDER BY timestamp DESC LIMIT 100"
    )
    .bind(token_address.into_inner())
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(trades) => HttpResponse::Ok().json(trades),
        Err(e) => {
            eprintln!("Error fetching trades: {:?}", e);
            HttpResponse::InternalServerError().body("DB error")
        }
    }
}

pub async fn get_tokens_created(
    pool: web::Data<PgPool>,
    account_id: web::Path<String>,
) -> HttpResponse {
    let result = sqlx::query_as::<_, CultToken>(
        "SELECT * FROM public.cult_token WHERE token_creator = $1 ORDER BY block_timestamp DESC"
    )
    .bind(account_id.into_inner())
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(tokens) => HttpResponse::Ok().json(tokens),
        Err(e) => {
            eprintln!("Error fetching tokens created by account: {:?}", e);
            HttpResponse::InternalServerError().body("DB error")
        }
    }
}


pub async fn get_account_details(
    pool: web::Data<PgPool>,
    account_id: web::Path<String>,
) -> HttpResponse {
    let result = sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE id = $1"
    )
    .bind(account_id.into_inner())
    .fetch_optional(pool.get_ref())
    .await;

    match result {
        Ok(Some(account)) => HttpResponse::Ok().json(account),
        Ok(None) => HttpResponse::NotFound().body("Account not found"),
        Err(e) => {
            eprintln!("Error fetching account: {:?}", e);
            HttpResponse::InternalServerError().body("DB error")
        }
    }
}