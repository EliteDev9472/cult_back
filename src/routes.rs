use alloy::primitives::U256;
use actix_web::{get, post, delete, put, web, HttpResponse, Responder};
use sqlx::{PgPool, Postgres, Transaction};
use serde::{Deserialize, Serialize};
use crate::models::{ CultTokenDataResponse, CommunityResponse, UpdateAccountRequest, WatchlistActionRequest, CreateAccountRequest, Account, PaginationParams, TopHolderParams, CultTokensResponse, CultTokenTopHolders, TokenTradesResponse, AccountDetailResponse,AccountData, CreatedToken, OwnedToken, WatchlistToken, Community, CreateAccountResponse};
use std::result::Result::Ok;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use rand::Rng;
use anyhow;
use utoipa::OpenApi;
use bigdecimal::ToPrimitive;

fn generate_referral_code() -> String {
    // Characters to use in the referral code (alphanumeric without confusing characters)
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    const CODE_LENGTH: usize = 8;
    
    let mut rng = rand::rng();
    let code: String = (0..CODE_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    
    code
}

async fn validate_referral_code(
    tx: &mut Transaction<'_, Postgres>,
    referral_code: &str
) -> Result<String, anyhow::Error> {
    let record = sqlx::query!(
        "SELECT id FROM account WHERE referral_code = $1",
        referral_code
    )
    .fetch_optional(&mut **tx)  
    .await?;
    
    record.map(|r| r.id).ok_or_else(|| anyhow::anyhow!("Invalid referral code"))
}

/////////////////////
/// ACCOUNT STUFF ////
/// //////////////////

/// Create a new user account
#[utoipa::path(
    tag = "Account",
    post,
    path = "/account",
    request_body(
        content = CreateAccountRequest,
        description = "Payload for creating a new account, including optional referral and social handles",
        content_type = "application/json"
    ),
    responses(
        (status = 201, description = "Account successfully created", body = CreateAccountResponse),
        (status = 400, description = "Invalid referral code"),
        (status = 409, description = "Account already exists"),
        (status = 500, description = "Internal server or database error")
    )
)]
#[post("/account")]
pub async fn create_account(
    pool: web::Data<PgPool>,
    request: web::Json<CreateAccountRequest>,
) -> impl Responder {
    let profile_data = match (&request.twitter, &request.discord) {
        (Some(twitter), Some(discord)) => Some((twitter.clone(), discord.clone())),
        _ => None,
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Failed to start transaction: {e}")),
    };

    // Check for existing account
    // Check for existing account
    match sqlx::query!("SELECT id FROM account WHERE id = $1", request.user_id)
        .fetch_optional(&mut *tx)
        .await
    {
        Ok(Some(_)) => return HttpResponse::Conflict().json("Account already exists"),
        Err(e) => return HttpResponse::InternalServerError().json(format!("Database error: {e}")),
        _ => (),
    };
    // Handle referral code validation
    let referrer_id = match &request.referral_code {
        Some(code) => match validate_referral_code(&mut tx, code).await {
            Ok(id) => Some(id),
            Err(e) => return HttpResponse::BadRequest().json(format!("Invalid referral code: {e}")),
        },
        None => None,
    };

    // Generate unique referral code with collision check
    let mut new_referral_code;
    loop {
        new_referral_code = generate_referral_code();
        match validate_referral_code(&mut tx, &new_referral_code).await {
            Ok(_) => continue, // Code exists, regenerate
            Err(_) => break,   // Unique code found
        }
    }
    

    // Insert new account
    match sqlx::query!(
        r#"
        INSERT INTO account (
            id, slug, referral_code, diamond_hand_probability, 
            referrer_id, total_referrals, fee_collected, twitter, discord
        )
        VALUES ($1, $2, $3, 0, $4, 0, 0, $5, $6)
        "#,
        request.user_id,
        None::<String>,
        new_referral_code,
        referrer_id,
        request.twitter.as_ref(),
        request.discord.as_ref()
    )
    .execute(&mut *tx)
    .await
    {
        Ok(_) => (),
        Err(e) => return HttpResponse::InternalServerError().body(format!("Failed to create account: {e}")),
    };

    
    // Update referrer's count
    if let Some(ref_id) = referrer_id {
        if let Err(e) = sqlx::query!(
            "UPDATE account SET total_referrals = total_referrals + 1 WHERE id = $1",
            ref_id
        )
        .execute(&mut *tx)
        .await
        {
            return HttpResponse::InternalServerError().body(format!("Failed to update referrals: {e}"));
        }
    }
    
    // Commit transaction
    if let Err(e) = tx.commit().await {
        return HttpResponse::InternalServerError().body(format!("Transaction commit failed: {e}"));
    }

    HttpResponse::Created().json(serde_json::json!({
        "user_id": request.user_id,
        "referral_code": new_referral_code,
    }))
}

// Get user account
#[utoipa::path(
    tag = "Account",
    get,
    path = "/account/{user_id}",
    params(
        ("user_id" = String, Path, description = "User identifier")
    ),
    responses(
        (status = 200, description = "Returns basic account data", body = Account),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Database error")
    )
)]
#[get("/account/{user_id}")]
pub async fn get_account(pool: web::Data<PgPool>, user_id: web::Path<String>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT 
            id, slug, referral_code, diamond_hand_probability,
            referrer_id, total_referrals, fee_collected::TEXT,
            twitter, discord, tokens_created, tokens_migrated
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
                fee_collected: row.fee_collected.map_or(0, |s| s.parse::<u128>().unwrap_or(0)),
                twitter: row.twitter,
                discord: row.discord,
                tokens_created: row.tokens_created.map(|v| v as u32).unwrap_or(0),
                tokens_migrated: row.tokens_migrated.map(|v| v as u32).unwrap_or(0)
            };

            HttpResponse::Ok().json(account)
        }
        Ok(None) => HttpResponse::NotFound().body("Account not found"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

#[utoipa::path(
    tag = "Account",
    get,
    path = "/accountData/{user_id}",
    params(
        ("user_id" = String, Path, description = "User identifier")
    ),
    responses(
         (status = 200, description = "Returns detailed account data", body = AccountData),
         (status = 404, description = "Account not found"),
         (status = 500, description = "Database error")
    )
)]
#[get("/accountData/{user_id}")]
pub async fn get_account_data(pool: web::Data<PgPool>, user_id: web::Path<String>) -> impl Responder {
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

    HttpResponse::Ok().json(AccountData {
        created_tokens: created,
        owned_tokens: owned,
        watchlist: watchlist,
        communities: communities,
    })
}


#[utoipa::path(
    tag = "Watchlist",
    post,
    path = "/account/watchlist",
    request_body(
        content = WatchlistActionRequest,
        description = "Token to add to account's watchlist",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "Token added to watchlist"),
        (status = 409, description = "Token already in watchlist"),
        (status = 500, description = "Database error")
    )
)]
#[post("/account/watchlist")]
pub async fn add_to_watchlist(
    pool: web::Data<PgPool>,
    request: web::Json<WatchlistActionRequest>,
) -> impl Responder {
    let result = sqlx::query!(
        "INSERT INTO account_watchlist (account_id, cult_token_id) VALUES ($1, $2)",
        request.account_id,
        request.cult_token_id
    )
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            if let Some(db_err) = e.as_database_error() {
                if db_err.message().contains("duplicate key") {
                    return HttpResponse::Conflict().body("Token already in watchlist");
                }
            }
            HttpResponse::InternalServerError().body(format!("Database error: {e}"))
        }
    }
}

#[utoipa::path(
    tag = "Watchlist",
    delete,
    path = "/account/watchlist",
    request_body(
        content = WatchlistActionRequest,
        description = "Token to remove from account's watchlist",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "Token removed from watchlist"),
        (status = 404, description = "Token not found in watchlist"),
        (status = 500, description = "Database error")
    )
)]
#[delete("/account/watchlist")]
pub async fn remove_from_watchlist(
    pool: web::Data<PgPool>,
    request: web::Json<WatchlistActionRequest>,
) -> impl Responder {
    let result = sqlx::query!(
        "DELETE FROM account_watchlist WHERE account_id = $1 AND cult_token_id = $2",
        request.account_id,
        request.cult_token_id
    )
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {e}"))
    }
}


// Update user account
#[utoipa::path(
    tag = "Account",
    put,
    path = "/account/update",
    request_body(
        content = UpdateAccountRequest,
        description = "Update Twitter and/or Discord handle for an account",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "Account updated successfully"),
        (status = 500, description = "Database error")
    )
)]
#[put("/account/update")]
pub async fn update_account(
    pool: web::Data<PgPool>,
    update: web::Json<UpdateAccountRequest>,
) -> impl Responder {
    let result = sqlx::query!(
        r#"
        UPDATE account
        SET 
            twitter = COALESCE($1, twitter),
            discord = COALESCE($2, discord)
        WHERE twitter = $1 OR discord = $2
        "#,
        update.twitter,
        update.discord
    )
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}


fn handle_error(e: sqlx::Error, context: &str) -> HttpResponse {
    eprintln!("{}: {}", context, e);
    HttpResponse::InternalServerError().finish()
}

/////////////////////
/// COMMUNITY STUFF ////
/// //////////////////

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


#[utoipa::path(
    tag = "Communities",
    get,
    path = "/communities",
    responses(
        (status = 200, description = "List of all communities", body = [CommunityResponse]),
        (status = 500, description = "Database error")
    )
)]
#[get("/communities")]
pub async fn get_all_communities(pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT name, img_url, chain, merkle_root, holder_count, community_score FROM communities
        "#
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(rows) => {
            let communities: Vec<CommunityResponse> = rows
                .into_iter()
                .map(|row| CommunityResponse {
                    name: row.name,
                    img_url: row.img_url,
                    chain: row.chain,
                    merkle_root: hex::encode(row.merkle_root),
                    holder_count: row.holder_count,
                    community_score: row.community_score.map(|bd| bd.to_f32().unwrap_or(0.0)),
                })
                .collect();

            HttpResponse::Ok().json(communities)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}


/////////////////////
/// TOKEN STUFF ////
/// //////////////////

/// Get a paginated list of cult tokens
#[utoipa::path(
    tag = "Tokens",
    get,
    path = "/cult/{offset}/{limit}",
    params(
        PaginationParams
    ),
    responses(
        (status = 200, description = "Returns a list of cult tokens", body = Vec<CultTokensResponse>),
        (status = 500, description = "Database error")
    )
)]
#[get("/cult/{offset}/{limit}")]
pub async fn get_cult_tokens(pool: web::Data<PgPool>, path: web::Path<PaginationParams>) -> impl Responder {
    let PaginationParams { offset, limit } = path.into_inner();

    let result = sqlx::query!(
        r#"
        SELECT 
            id,
            token_creator,
            name,
            symbol,
            ipfs_content,
            holder_count,
            market_cap,
            volume,
            total_airdrop_recipient_count,
            creator_holdings,
            top_holders,
            buy_tx_count_1h,
            sell_tx_count_1h,
            last_traded,
            bonding_curve_percentage
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
        Ok(rows) => {
            let tokens: Vec<CultTokensResponse> = rows
                .into_iter()
                .map(|row| CultTokensResponse {
                    token_address: row.id,
                    token_creator: row.token_creator,
                    name: row.name,
                    symbol: row.symbol,
                    ipfs_data: row.ipfs_content,
                    holder_count: row.holder_count as u32,
                    market_cap: row.market_cap.to_f64().unwrap_or(0.0),
                    volume: row.volume.to_f64().unwrap_or(0.0),
                    total_airdrop_recipient_count: row.total_airdrop_recipient_count as u32,
                    creator_holdings: row.creator_holdings.to_f64().unwrap_or(0.0),
                    top_holders: row.top_holders.to_f64().unwrap_or(0.0),
                    buy_tx_count_1h: row.buy_tx_count_1h as u32,
                    sell_tx_count_1h: row.sell_tx_count_1h as u32,
                    last_traded: Some(row.last_traded),
                    bonding_curve_percentage: row.bonding_curve_percentage.to_f64().unwrap_or(0.0),
                })
                .collect();

            HttpResponse::Ok().json(tokens)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

// pub async fn get_top_coins(pool: web::Data<PgPool>) -> impl Responder {

//     let result = sqlx::query!(
//         r#"
//         SELECT 
//             id,
//             token_creator,
//             name,
//             symbol,
//             ipfs_content
//         FROM cult_token ORDER BY holder_count DESC LIMIT 3
//         "#
//     )
//     .fetch_optional(pool.get_ref())
//     .await;


//     match result {
//         Ok(Some(row)) => {
//             let response = CultTokensResponse {
//                 token_address: row.id,
//                 token_creator: row.token_creator,
//                 name: row.name,
//                 symbol: row.symbol,
//                 ipfsData: row.ipfs_content,
//             };

//             HttpResponse::Ok().json(response)
//         }
//         Ok(None) => HttpResponse::NotFound().body("Account not found"),
//         Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
//     }
// }


#[utoipa::path(
    tag = "Tokens",
    get,
    path = "/cult/{address}",
    params(
        ("address" = String, Path, description = "Token address (id)")
    ),
    responses(
        (status = 200, description = "Returns token metadata", body = CultTokenDataResponse),
        (status = 404, description = "Token not found"),
        (status = 500, description = "Database error")
    )
)]
#[get("/cult/{address}")]
pub async fn get_cult_data(
    pool: web::Data<PgPool>,
    token_address: web::Path<String>,
) -> impl Responder {
    let result = sqlx::query!(
        r#"
        SELECT 
            id,
            token_creator,
            name,
            symbol,
            ipfs_content,
            holder_count,
            market_cap,
            volume,
            total_airdrop_recipient_count,
            creator_holdings,
            top_holders,
            buy_tx_count_1h,
            sell_tx_count_1h,
            last_traded,
            bonding_curve_percentage,
            is_graduated,
            pool_address
        FROM cult_token 
        WHERE id = $1
        "#,
        token_address.into_inner()
    )
    .fetch_optional(pool.get_ref())
    .await;

    match result {
        Ok(Some(row)) => {
            let response = CultTokenDataResponse {
                id: row.id,
                token_creator: row.token_creator,
                name: row.name,
                symbol: row.symbol,
                ipfs_content: row.ipfs_content,
                holder_count: row.holder_count as u32,
                market_cap: row.market_cap.to_f64().unwrap_or(0.0),
                volume: row.volume.to_f64().unwrap_or(0.0),
                total_airdrop_recipient_count: row.total_airdrop_recipient_count as u32,
                creator_holdings: row.creator_holdings.to_f64().unwrap_or(0.0),
                top_holders: row.top_holders.to_f64().unwrap_or(0.0),
                buy_tx_count_1h: row.buy_tx_count_1h as u32,
                sell_tx_count_1h: row.sell_tx_count_1h as u32,
                last_traded: Some(row.last_traded),
                bonding_curve_percentage: row.bonding_curve_percentage.to_f64().unwrap_or(0.0),
                is_graduated: row.is_graduated,
                pool_address: row.pool_address,
            };

            HttpResponse::Ok().json(response)
        }
        Ok(None) => HttpResponse::NotFound().body("Token not found"),
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
        Ok(None) => HttpResponse::NotFound().body("Account not found"),
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


//
// ----- OpenAPI Aggregation -----
//

#[derive(OpenApi)]
#[openapi(
    paths(
        create_account,
        get_account,
        get_account_data,
        add_to_watchlist,
        remove_from_watchlist,
        update_account, 
        get_all_communities,
        
        get_cult_tokens,
        get_cult_data,

    ),
    components(
        schemas(
            CreateAccountRequest,
            CreateAccountResponse,
            Account,
            AccountData,
            WatchlistActionRequest,
            UpdateAccountRequest, 
            CommunityResponse,
            CultTokensResponse,
            CultTokenDataResponse,
        )
    ),
    tags(
        (name = "Account", description = "Payload for creating, getting account, including optional referral and social handles"),
        (name = "Account", description = "Returns detailed account data"),
        (name = "Watchlist", description = "add or remove tokens from or to account's watchlist"),
        (name = "Tokens", description = "Returns token data for home page discover cards"),
        (name = "Communities", description = "View all supported communities")
    )
)]
pub struct ApiDoc;