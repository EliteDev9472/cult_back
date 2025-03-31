use alloy::primitives::U256;
use actix_web::{web, HttpResponse, Responder};
use sqlx::{postgres::Postgres, PgPool};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::models::{CultToken, TokenTrade, Account, TokenBalance, PaginationParams, TopHolderParams, CultTokensResponse, CultTokenTopHolders, CultTokensDataResponse, TokenTradesResponse, AccountDetailResponse, TokenCreatedResponse};
use alloy::signers::k256::elliptic_curve::pkcs8::der::asn1::Int;
use sqlx::{Pool};
use std::result::Result::Ok;
use bigdecimal::BigDecimal;
use std::str::FromStr;


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


pub async fn get_cult_tokens(pool: web::Data<PgPool>, path: web::Path<PaginationParams>) -> impl Responder {
    let PaginationParams { offset, limit } = path.into_inner();

    let result = sqlx::query!(
        r#"
        SELECT 
            id,
            token_creator,
            name,
            symbol,
            ipfs_content
        FROM cult_token
        ORDER BY block_timestamp DESC
        OFFSET $1 LIMIT $2
        "#,
        offset,
        limit
    )
    .fetch_optional(pool.get_ref())
    .await;

    match result {
        Ok(Some(row)) => {
            let response = CultTokensResponse {
                token_address: row.id,
                token_creator: row.token_creator,
                name: row.name,
                symbol: row.symbol,
                ipfsData: row.ipfs_content,
            };

            HttpResponse::Ok().json(response)
        }
        Ok(None) => HttpResponse::NotFound().body("Profile not found"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

pub async fn get_top_coins(pool: web::Data<PgPool>) -> impl Responder {

    let result = sqlx::query!(
        r#"
        SELECT 
            id,
            token_creator,
            name,
            symbol,
            ipfs_content
        FROM cult_token ORDER BY holder_count DESC LIMIT 3
        "#
    )
    .fetch_optional(pool.get_ref())
    .await;


    match result {
        Ok(Some(row)) => {
            let response = CultTokensResponse {
                token_address: row.id,
                token_creator: row.token_creator,
                name: row.name,
                symbol: row.symbol,
                ipfsData: row.ipfs_content,
            };

            HttpResponse::Ok().json(response)
        }
        Ok(None) => HttpResponse::NotFound().body("Profile not found"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}


pub async fn get_token_data(pool: web::Data<PgPool>, token_address: web::Path<String>) -> impl Responder {

    let result = sqlx::query!(
        r#"
        SELECT 
            id,
            token_creator,
            bonding_curve,
            name,
            symbol,
            pool_address,
            block_timestamp,
            holder_count,
            airdrop_contract,
            ipfs_content
        FROM cult_token WHERE id = $1
        "#,
        token_address.into_inner()
    )
    .fetch_optional(pool.get_ref())
    .await;


    match result {
        Ok(Some(row)) => {
            let result = CultTokensDataResponse {
                id: row.id,
                token_creator: row.token_creator,
                bonding_curve: row.bonding_curve,
                name: row.name,
                symbol: row.symbol,
                pool_address: row.pool_address,
                block_timestamp: row.block_timestamp,
                holder_count: row.holder_count,
                airdrop_contract: row.airdrop_contract,
                ipfs_content: row.ipfs_content,
            };

            HttpResponse::Ok().json(result)
        }
        Ok(None) => HttpResponse::NotFound().body("Profile not found"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}


pub async fn get_top_holders(pool: web::Data<PgPool>, path: web::Path<TopHolderParams>) -> impl Responder {

    let TopHolderParams { token_address, offset, limit } = path.into_inner();

    let result = sqlx::query!(
        r#"
        SELECT 
            account_id,
            value_z
        FROM token_balance WHERE token_id = $1
        ORDER BY value_z DESC offset $2 limit $3
        "#,
        token_address,
        offset,
        limit
    )
    .fetch_optional(pool.get_ref())
    .await;


    match result {
        Ok(rows) => {
            let holders: Vec<CultTokenTopHolders> = rows
                .into_iter()
                .map(|row| CultTokenTopHolders {
                    id: row.account_id,
                    value: row.value_z.unwrap_or_else(|| BigDecimal::from_str("0").unwrap()),
                })
                .collect();

            HttpResponse::Ok().json(holders)
        }
        Ok(None) => HttpResponse::NotFound().body("Profile not found"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}



pub async fn get_token_trades(
    pool: web::Data<sqlx::PgPool>,
    path: web::Path<TopHolderParams>,
) -> impl Responder {
    let TopHolderParams { token_address, offset, limit } = path.into_inner();

    let result = sqlx::query_as!(
        TokenTradesResponse,
        r#"
        SELECT 
            token_id as id,
            trader_id as trader,
            recipient_id as recipient,
            order_referrer_id as "orderReferrer",
            eth_amount as "ethAmount?",
            token_amount as "tokenAmount?",
            trader_token_balance as "traderTokenBalance?",
            market_type as "marketType",
            timestamp,
            transaction_hash as "transactionHash"
        FROM token_trade 
        WHERE token_id = $1
        ORDER BY timestamp DESC 
        OFFSET $2 LIMIT $3
        "#,
        token_address,
        offset,
        limit
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            eprintln!("Error fetching trades: {:?}", e);
            HttpResponse::InternalServerError().body("DB error")
        }
    }
}


pub async fn get_account_details(
    pool: web::Data<PgPool>,
    account_id: web::Path<String>,
) -> HttpResponse {

    let result = sqlx::query_as!(
        AccountDetailResponse,
        r#"
        SELECT 
            id,
            slug,
            diamond_hand_probability,
            total_referrals,
            fee_collected as "feeCollected?"
        FROM public.account WHERE id = $1
        "#,
        account_id.into_inner()
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(rows) => HttpResponse::Ok().json(rows),
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

    let result = sqlx::query_as!(
        TokenCreatedResponse,
        r#"
            SELECT
                account.id,
                slug,
                diamond_hand_probability,
                fee_collected as "feeCollected?",
                total_referrals,
                cult_token.id as "token_id?",
                cult_token.name as "token_name?",
                cult_token.symbol as "token_symbol?",
                ipfs_content as "ipfs_content?",
                value_z as "value?"
            FROM account 
            LEFT JOIN cult_token ON account.id = cult_token.token_creator
            LEFT JOIN token_balance ON account.id = token_balance.account_id
            WHERE account.id = $1
        "#,
        account_id.into_inner()
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(e) => {
            eprintln!("Error fetching trades: {:?}", e);
            HttpResponse::InternalServerError().body("DB error")
        }
    }
}