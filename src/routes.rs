use alloy::primitives::U256;
use actix_web::{web, HttpResponse, Responder};
use sqlx::{PgPool, postgres::Postgres};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::models::{CultToken, TokenTrade, Account};

// // Get all tokens
// pub async fn get_tokens(pool: web::Data<PgPool>) -> impl Responder {
//     let result: Result<Vec<CultToken>, sqlx::Error> = sqlx::query_as!(
//         CultToken,
//         r#"
//         SELECT 
//             id, factory_address, token_creator, protocol_fee_recipient,
//             bonding_curve, token_uri, name, symbol, token_address,
//             pool_address, block_number, block_timestamp, transaction_hash,
//             holder_count, airdrop_contract
//         FROM cult_token
//         ORDER BY holder_count DESC
//         "#
//     )
//     .fetch_all(&**pool)
//     .await;

//     match result {
//         Ok(tokens) => HttpResponse::Ok().json(tokens),
//         Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
//     }
// }

// // Get specific token by ID
// pub async fn get_token(pool: web::Data<PgPool>, token_id: web::Path<String>) -> impl Responder {
//     let result: Result<Option<CultToken>, sqlx::Error> = sqlx::query_as!(
//         CultToken,
//         r#"
//         SELECT 
//             id, factory_address, token_creator, protocol_fee_recipient,
//             bonding_curve, token_uri, name, symbol, token_address,
//             pool_address, block_number, block_timestamp, transaction_hash,
//             holder_count, airdrop_contract
//         FROM cult_token
//         WHERE token_address = $1
//         "#,
//         token_id.into_inner()
//     )
//     .fetch_optional(&**pool)
//     .await;

//     match result {
//         Ok(Some(token)) => HttpResponse::Ok().json(token),
//         Ok(None) => HttpResponse::NotFound().body("Token not found"),
//         Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
//     }
// }

// // Get trade data for a specific token
// pub async fn get_token_trades(
//     pool: web::Data<PgPool>,
//     token_id: web::Path<String>,
// ) -> impl Responder {
//     let result: Result<Vec<TokenTrade>, sqlx::Error> = sqlx::query_as!(
//         TokenTrade,
//         r#"
//         SELECT 
//             id, token_id, trade_type, trader_id, recipient_id,
//             order_referrer_id, total_eth, eth_fee, eth_amount,
//             token_amount, trader_token_balance, total_supply,
//             market_type, timestamp, transaction_hash
//         FROM token_trade
//         WHERE token_id = $1
//         ORDER BY timestamp DESC
//         LIMIT 100
//         "#,
//         token_id.into_inner()
//     )
//     .fetch_all(&**pool)
//     .await;

//     match result {
//         Ok(trades) => HttpResponse::Ok().json(trades),
//         Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
//     }
// }

// Get user profile
pub async fn get_profile(pool: web::Data<PgPool>, user_id: web::Path<String>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT 
            id, slug, referral_code, diamond_hand_probability,
            referrer_id, total_referrals, fee_collected,
            twitter, discord
        FROM account
        WHERE id = $1
        "#,
        user_id.into_inner()
    )
    .fetch_optional(pool.get_ref())
    .await;


    match result {
        Ok(Some(row)) => {


            // let fee_collected = match row.fee_collected {
            //     Some(fee) if fee >= 0 => U256::from(fee as u64),
            //     _ => U256::zero(), // Covers None or negative values
            // };

            let account = Account {
                id: Some(row.id),
                slug: row.slug,
                referral_code: row.referral_code,
                diamond_hand_probability: row.diamond_hand_probability as u32,
                referrer_id: row.referrer_id,
                total_referrals: row.total_referrals.map(|v| v as u32),
                fee_collected: U256::from(324 as u64),
                twitter: row.twitter,
                discord: row.discord,
            };

            HttpResponse::Ok().json(account)
        }
        Ok(None) => HttpResponse::NotFound().body("Profile not found"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ProfileData {
    created_tokens: Vec<CreatedToken>,
    owned_tokens: Vec<OwnedToken>,
    watchlist: Vec<WatchlistToken>,
    communities: Vec<Community>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreatedToken {
    id: String,
    name: String,
    symbol: String,
    ipfs_content: String,
    user_balance: String,  // Wei as string
}

#[derive(Debug, Serialize, Deserialize)]
struct OwnedToken {
    id: String,
    name: String,
    symbol: String,
    ipfs_content: String,
    user_balance: String,  // Wei as string
}

#[derive(Debug, Serialize, Deserialize)]
struct WatchlistToken {
    id: String,
    name: String,
    symbol: String,
    ipfs_content: String,
    user_balance: String,  // Wei as string
}

#[derive(Debug, Serialize, Deserialize)]
struct Community {
    id: String,
    name: String,
    img_url: String,
}

pub async fn get_profile_data(pool: web::Data<PgPool>, user_id: web::Path<String>) -> impl Responder {
    let user_address = user_id.into_inner();
    
    let mut conn = match pool.acquire().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to acquire connection: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };


// Created tokens with balance
let created = match sqlx::query_as!(
    CreatedToken,
    r#"
    SELECT 
        ct.id,
        ct.name,
        ct.symbol,
        ct.token_uri as ipfs_content,
        COALESCE(tb.holdings_value::TEXT, '0') as "user_balance!"
    FROM cult_token ct
    LEFT JOIN token_balance tb 
        ON ct.id = tb.token_id 
        AND tb.account_id = $1
    WHERE ct.token_creator = $1
    "#,
    user_address
)
.fetch_all( &mut *conn)
.await {
    Ok(tokens) => tokens,
    Err(e) => return handle_error(e, "Error fetching created tokens"),
};

// Owned tokens
let owned = match sqlx::query_as!(
    OwnedToken,
    r#"
    SELECT 
        ct.id,
        ct.name,
        ct.symbol,
        ct.token_uri as ipfs_content,
        tb.holdings_value::TEXT as "user_balance!"
    FROM token_balance tb
    JOIN cult_token ct ON tb.token_id = ct.id
    WHERE tb.account_id = $1
    "#,
    user_address
)
.fetch_all( &mut *conn)
.await {
    Ok(tokens) => tokens,
    Err(e) => return handle_error(e, "Error fetching owned tokens"),
};

// Watchlist tokens with balance
let watchlist = match sqlx::query_as!(
    WatchlistToken,
    r#"
    SELECT 
        ct.id,
        ct.name,
        ct.symbol,
        ct.token_uri as ipfs_content,
        COALESCE(tb.holdings_value::TEXT, '0') as "user_balance!"
    FROM account_watchlist aw
    JOIN cult_token ct ON aw.cult_token_id = ct.id
    LEFT JOIN token_balance tb 
        ON ct.id = tb.token_id 
        AND tb.account_id = $1
    WHERE aw.account_id = $1
    "#,
    user_address
)
.fetch_all( &mut *conn)
.await {
    Ok(tokens) => tokens,
    Err(e) => return handle_error(e, "Error fetching watchlist"),
};
    // Communities
    let communities = match sqlx::query_as!(
        Community,
        r#"
        SELECT 
            c.id,
            c.name,
            c.img_url
        FROM account_communities ac
        JOIN communities c ON ac.community_id = c.id
        WHERE ac.account_id = $1
        "#,
        user_address
    )
    .fetch_all( &mut *conn)
    .await {
        Ok(communities) => communities,
        Err(e) => return handle_error(e, "Error fetching communities"),
    };

    HttpResponse::Ok().json(ProfileData {
        created_tokens: created,
        owned_tokens: owned,
        watchlist: watchlist,
        communities: communities,
    })
}

fn handle_error(e: sqlx::Error, context: &str) -> HttpResponse {
    eprintln!("{}: {}", context, e);
    HttpResponse::InternalServerError().finish()
}
// Update user profile
#[derive(Debug, Deserialize)]
pub struct UpdateProfile {
    twitter: Option<String>,
    discord: Option<String>,
}

// pub async fn update_profile(
//     pool: web::Data<PgPool>,
//     user_id: web::Path<String>,
//     update: web::Json<UpdateProfile>,
// ) -> impl Responder {
//     let result: Result<Account, sqlx::Error> = sqlx::query_as!(
//         Account,
//         r#"
//         UPDATE account
//         SET 
//             twitter = COALESCE($1, twitter),
//             discord = COALESCE($2, discord)
//         WHERE id = $3
//         RETURNING 
//             id, slug, referral_code, diamond_hand_probability,
//             referrer_id, total_referrals, fee_collected,
//             twitter, discord
//         "#,
//         update.twitter,
//         update.discord,
//         user_id.into_inner()
//     )
//     .fetch_one(&**pool)
//     .await;

//     match result {
//         Ok(account) => HttpResponse::Ok().json(account),
//         Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
//     }
// }

pub async fn get_diamond_hands(pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT account_id FROM diamond_hand_list
        "#
    )
    .fetch_all(pool.get_ref()) // Use fetch_all for multiple rows
    .await;

    match result {
        Ok(rows) => {
            let accounts: Vec<String> = rows.into_iter().map(|row| row.account_id).collect();
            HttpResponse::Ok().json(accounts)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

#[derive(Serialize)]
struct CommunityResponse {
    name: String,
    img_url: String,
    chain: String,
    merkle_root: Option<Vec<u8>>,
}

pub async fn get_all_communities(pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT name, img_url, chain, merkle_root FROM communities
        "#
    )
    .fetch_all(pool.get_ref()) // Fetch all rows
    .await;

    match result {
        Ok(rows) => {
            let communities: Vec<CommunityResponse> = rows
                .into_iter()
                .map(|row| CommunityResponse {
                    name: row.name,
                    img_url: row.img_url,
                    chain: row.chain,
                    merkle_root: row.merkle_root, // Correctly mapped as Option<Vec<u8>>
                })
                .collect();

            HttpResponse::Ok().json(communities)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}