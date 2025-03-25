use anyhow::Ok;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use std::time::SystemTime;
use reqwest;

use crate::models::{Account, CultToken, TokenBalance, TokenIPFSData, TokenTrade, TradeType};

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

// pub async fn handle_cult_token_created(
//     event: CultTokenCreatedEvent,
//     pool: &Pool<Postgres>,
// ) -> Result<(), anyhow::Error> {
//     log::info!("CultTokenCreated: {}", event.token_address);

//     // Load or create accounts
//     let token_creator = load_or_create_account(pool, &event.token_creator, None).await?;
//     let airdrop_contract = load_or_create_account(pool, &event.airdrop_contract, Some("Airdrop Contract".to_string())).await?;

//     // Create a new cult token
//     let cult_token = CultToken {
//         id: None,
//         factory_address: event.factory_address.clone(),
//         token_creator: token_creator.id.unwrap().to_string(),
//         protocol_fee_recipient: event.protocol_fee_recipient.clone(),
//         bonding_curve: event.bonding_curve.clone(),
//         token_uri: event.token_uri.clone(),
//         name: event.name.clone(),
//         symbol: event.symbol.clone(),
//         token_address: event.token_address.clone(),
//         pool_address: event.pool_address.clone(),
//         block_number: event.block_number as i64,
//         block_timestamp: Utc.timestamp_opt(event.block_timestamp as i64, 0).unwrap(),
//         transaction_hash: event.block_hash.clone(),
//         holder_count: 0,
//         airdrop_contract: airdrop_contract.id.unwrap().to_string(),
//         trades: Some(vec![]),
//         balances: Some(vec![]),
//         ipfs_data: Some(vec![]),    
//     };

//     // Insert the new cult token
//     let token_id = insert_cult_token(pool, &cult_token).await?;

//     // Handle IPFS data
//     let ipfs_prefix = "ipfs://";
//     if let Some(ipfs_index) = cult_token.token_uri.find(ipfs_prefix) {
//         let hash = &cult_token.token_uri[(ipfs_index + ipfs_prefix.len())..];
        
//         let client = reqwest::Client::new();
//         let response = client
//             .get(&format!("https://ipfs.io/ipfs/{}", hash))
//             .header("Accept", "application/json")
//             .send()
//             .await?;
        
//         if response.status().is_success() {
//             let content = response.text().await?;
//             let ipfs_data = TokenIPFSData {
//                 id: None,
//                 hash: hash.to_string(),
//                 content,
//                 token_id: token_id,
//             };
            
//             insert_token_ipfs_data(pool, &ipfs_data).await?;
//         }
//     }

//     Ok(())
// }

// pub async fn handle_cult_token_buy(
//     event: CultTokenBuyEvent,
//     pool: &Pool<Postgres>,
// ) -> Result<(), anyhow::Error> {
//     log::info!("CultTokenBuy: {}, {}", event.buyer, event.recipient);

//     let trader = load_or_create_account(pool, &event.buyer, None).await?;
//     let recipient = load_or_create_account(pool, &event.recipient, None).await?;
//     let order_referrer = load_or_create_account(pool, &event.order_referrer, None).await?;

//     let token_trade = TokenTrade {
//         id: None,
//         token_id: get_token_id_by_address(pool, &event.srcAddress).await?,
//         trade_type: TradeType::Buy,
//         trader_id: trader.id.unwrap(),
//         recipient_id: recipient.id.unwrap(),
//         order_referrer_id: order_referrer.id.unwrap(),
//         total_eth: event.total_eth,
//         eth_fee: event.eth_fee,
//         eth_amount: event.eth_sold,
//         token_amount: event.tokens_bought,
//         trader_token_balance: event.buyer_token_balance,
//         total_supply: event.total_supply,
//         market_type: event.market_type as i64,
//         timestamp: Utc.timestamp_opt(event.block_timestamp as i64, 0).unwrap(),
//         transaction_hash: event.block_hash.clone(),
//     };

//     insert_token_trade(pool, &token_trade).await?;

//     // Update token balance
//     let cult_token = get_cult_token_by_address(pool, &token_address).await?;
//     update_token_balance(
//         pool,
//         &cult_token,
//         trader.id.unwrap(),
//         event.buyer_token_balance,
//     ).await?;

//     Ok(())
// }

// pub async fn handle_cult_token_sell(
//     event: CultTokenSellEvent,
//     token_address: String,
//     pool: &Pool<Postgres>,
// ) -> Result<(), anyhow::Error> {
//     let trader = load_or_create_account(pool, &event.seller, None).await?;
//     let recipient = load_or_create_account(pool, &event.recipient, None).await?;
//     let order_referrer = load_or_create_account(pool, &event.order_referrer, None).await?;

//     let token_trade = TokenTrade {
//         id: None,
//         token_id: get_token_id_by_address(pool, &token_address).await?,
//         trade_type: TradeType::Sell,
//         trader_id: trader.id.unwrap(),
//         recipient_id: recipient.id.unwrap(),
//         order_referrer_id: order_referrer.id.unwrap(),
//         total_eth: event.total_eth,
//         eth_fee: event.eth_fee,
//         eth_amount: event.eth_bought,
//         token_amount: event.tokens_sold,
//         trader_token_balance: event.seller_token_balance,
//         total_supply: event.total_supply,
//         market_type: event.market_type as i64,
//         timestamp: Utc.timestamp_opt(event.block_timestamp as i64, 0).unwrap(),
//         transaction_hash: event.block_hash.clone(),
//     };

//     insert_token_trade(pool, &token_trade).await?;

//     // Update token balance
//     let cult_token = get_cult_token_by_address(pool, &token_address).await?;
//     update_token_balance(
//         pool,
//         &cult_token,
//         trader.id.unwrap(),
//         event.seller_token_balance,
//     ).await?;

//     Ok(())
// }

// pub async fn handle_cult_token_transfer(
//     event: CultTokenTransferEvent,
//     token_address: String,
//     pool: &Pool<Postgres>,
// ) -> Result<(), anyhow::Error> {
//     log::info!("TRANSFERING TOKEN");
    
//     let token = match get_cult_token_by_address(pool, &token_address).await {
//         Ok(token) => token,
//         Err(_) => return Ok(()), // If token doesn't exist, just return
//     };

//     let from = load_or_create_account(pool, &event.from, None).await?;
//     let to = load_or_create_account(pool, &event.to, None).await?;

//     if from.id.unwrap().to_string() != "0x0000000000000000000000000000000000000000" {
//         update_token_balance(
//             pool,
//             &token,
//             from.id.unwrap(),
//             event.from_token_balance,
//         ).await?;
//     }

//     update_token_balance(
//         pool,
//         &token,
//         to.id.unwrap(),
//         event.to_token_balance,
//     ).await?;

//     Ok(())
// }

// pub async fn handle_cult_token_fees(
//     event: CultTokenFeesEvent,
//     pool: &Pool<Postgres>,
// ) -> Result<(), anyhow::Error> {
//     log::info!("CultTokenFees: {}", event.order_referrer);

//     let order_referrer = load_or_create_account(pool, &event.order_referrer, None).await?;
    
//     // Get existing account details
//     let order_referrer_account = get_account_by_id(pool, order_referrer.id.unwrap()).await?;
    
//     // Update account
//     let updated_account = Account {
//         id: order_referrer.id,
//         slug: order_referrer_account.slug.or(Some("Order Referrer Fees".to_string())),
//         diamond_hand_probability: order_referrer_account.diamond_hand_probability,
//         referrer_id: order_referrer_account.referrer_id,
//         total_referrals: Some(order_referrer_account.total_referrals.unwrap_or(0) + 1),
//         fee_collected: order_referrer_account.fee_collected + event.order_referrer_fee,
//     };
    
//     update_account(pool, &updated_account).await?;

//     Ok(())
// }

// // Helper functions

// async fn load_or_create_account(
//     pool: &Pool<Postgres>,
//     address: &str,
//     slug: Option<String>,
// ) -> Result<Account, anyhow::Error> {
//     if let Ok(account) = get_account_by_address(pool, address).await {
//         return Ok(account);
//     }

//     let account = Account {
//         id: None,
//         slug,
//         diamond_hand_probability: 0,
//         referrer_id: None,
//         total_referrals: Some(0),
//         fee_collected: 0,
//     };

//     let id = insert_account(pool, &account).await?;
//     Ok(Account { id: Some(id), ..account })
// }

// async fn update_token_balance(
//     pool: &Pool<Postgres>,
//     token: &CultToken,
//     account_id: i64,
//     new_value: i64,
// ) -> Result<(), anyhow::Error> {
//     log::info!("UPDATING TOKEN BALANCE");
    
//     let token_id = token.id.unwrap();
//     let mut old_value: i64 = 0;
    
//     // Try to get existing balance
//     match get_token_balance(pool, token_id, account_id).await {
//         Ok(balance) => {
//             old_value = balance.value;
            
//             // Update existing balance
//             let now = Utc::now();
//             let mut updated_balance = balance;
//             updated_balance.value = new_value;
            
//             // If selling tokens, update the last_sold timestamp
//             if new_value < old_value {
//                 updated_balance.last_sold = now;
//                 updated_balance.held_for = (now - updated_balance.last_bought).num_seconds();
//             }
            
//             update_token_balance_db(pool, &updated_balance).await?;
//         },
//         Err(_) => {
//             // Create new balance
//             let balance = TokenBalance {
//                 id: None,
//                 token_id,
//                 account_id,
//                 value: new_value,
//                 last_bought: Utc::now(),
//                 last_sold: Utc::now(), // Set to the same as last_bought initially
//                 held_for: 0,
//             };
            
//             insert_token_balance(pool, &balance).await?;
//         }
//     }
    
//     // Update holder count on the token
//     let mut updated_token = token.clone();

//     if old_value == 0 && new_value > 0 {
//         updated_token.holder_count += 1;
//     }
//     // Holder lost all tokens (old_value > 0, new_value = 0)
//     else if old_value > 0 && new_value == 0 {
//         updated_token.holder_count -= 1;
//     }
    
//     update_cult_token(pool, &updated_token).await?;
    
//     Ok(())
// }

// // Database functions

// async fn insert_cult_token(pool: &Pool<Postgres>, token: &CultToken) -> Result<i64, anyhow::Error> {
//     let record = sqlx::query!(
//         r#"
//         INSERT INTO cult_token (
//             factory_address, token_creator, protocol_fee_recipient, bonding_curve,
//             token_uri, name, symbol, token_address, pool_address, block_number,
//             block_timestamp, transaction_hash, holder_count, airdrop_contract
//         )
//         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
//         RETURNING id
//         "#,
//         token.factory_address,
//         token.token_creator,
//         token.protocol_fee_recipient,
//         token.bonding_curve,
//         token.token_uri,
//         token.name,
//         token.symbol,
//         token.token_address,
//         token.pool_address,
//         token.block_number,
//         token.block_timestamp,
//         token.transaction_hash,
//         token.holder_count,
//         token.airdrop_contract
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record.id)
// }

// async fn update_cult_token(pool: &Pool<Postgres>, token: &CultToken) -> Result<(), anyhow::Error> {
//     sqlx::query!(
//         r#"
//         UPDATE cult_token
//         SET holder_count = $1
//         WHERE id = $2
//         "#,
//         token.holder_count,
//         token.id
//     )
//     .execute(pool)
//     .await?;
    
//     Ok(())
// }

// async fn get_cult_token_by_address(pool: &Pool<Postgres>, address: &str) -> Result<CultToken, anyhow::Error> {
//     let record = sqlx::query_as!(
//         CultToken,
//         r#"
//         SELECT * FROM cult_token 
//         WHERE token_address = $1
//         "#,
//         address
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record)
// }

// async fn get_token_id_by_address(pool: &Pool<Postgres>, address: &str) -> Result<i64, anyhow::Error> {
//     let record = sqlx::query!(
//         r#"
//         SELECT id FROM cult_token 
//         WHERE token_address = $1
//         "#,
//         address
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record.id)
// }

// async fn insert_token_ipfs_data(pool: &Pool<Postgres>, data: &TokenIPFSData) -> Result<i64, anyhow::Error> {
//     let record = sqlx::query!(
//         r#"
//         INSERT INTO token_ipfs_data (hash, content, token_id)
//         VALUES ($1, $2, $3)
//         RETURNING id
//         "#,
//         data.hash,
//         data.content,
//         data.token_id
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record.id)
// }

// async fn insert_account(pool: &Pool<Postgres>, account: &Account) -> Result<i64, anyhow::Error> {
//     let record = sqlx::query!(
//         r#"
//         INSERT INTO account (slug, diamond_hand_probability, referrer_id, total_referrals, fee_collected)
//         VALUES ($1, $2, $3, $4, $5)
//         RETURNING id
//         "#,
//         account.slug,
//         account.diamond_hand_probability,
//         account.referrer_id,
//         account.total_referrals,
//         account.fee_collected
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record.id)
// }

// async fn update_account(pool: &Pool<Postgres>, account: &Account) -> Result<(), anyhow::Error> {
//     sqlx::query!(
//         r#"
//         UPDATE account
//         SET slug = $1, diamond_hand_probability = $2, referrer_id = $3, 
//             total_referrals = $4, fee_collected = $5
//         WHERE id = $6
//         "#,
//         account.slug,
//         account.diamond_hand_probability,
//         account.referrer_id,
//         account.total_referrals,
//         account.fee_collected,
//         account.id
//     )
//     .execute(pool)
//     .await?;
    
//     Ok(())
// }

// async fn get_account_by_address(pool: &Pool<Postgres>, address: &str) -> Result<Account, anyhow::Error> {
//     let record = sqlx::query_as!(
//         Account,
//         r#"
//         SELECT * FROM account 
//         WHERE id = $1
//         "#,
//         address
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record)
// }

// async fn get_account_by_id(pool: &Pool<Postgres>, id: i64) -> Result<Account, anyhow::Error> {
//     let record = sqlx::query_as!(
//         Account,
//         r#"
//         SELECT * FROM account 
//         WHERE id = $1
//         "#,
//         id
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record)
// }

// async fn insert_token_trade(pool: &Pool<Postgres>, trade: &TokenTrade) -> Result<i64, anyhow::Error> {
//     let trade_type = match trade.trade_type {
//         TradeType::Buy => "BUY",
//         TradeType::Sell => "SELL",
//     };

//     let record = sqlx::query!(
//         r#"
//         INSERT INTO token_trade (
//             token_id, trade_type, trader_id, recipient_id, order_referrer_id,
//             total_eth, eth_fee, eth_amount, token_amount, trader_token_balance,
//             total_supply, market_type, timestamp, transaction_hash
//         )
//         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
//         RETURNING id
//         "#,
//         trade.token_id,
//         trade_type,
//         trade.trader_id,
//         trade.recipient_id,
//         trade.order_referrer_id,
//         trade.total_eth,
//         trade.eth_fee,
//         trade.eth_amount,
//         trade.token_amount,
//         trade.trader_token_balance,
//         trade.total_supply,
//         trade.market_type,
//         trade.timestamp,
//         trade.transaction_hash
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record.id)
// }

// async fn get_token_balance(pool: &Pool<Postgres>, token_id: i64, account_id: i64) -> Result<TokenBalance, anyhow::Error> {
//     let record = sqlx::query_as!(
//         TokenBalance,
//         r#"
//         SELECT * FROM token_balance 
//         WHERE token_id = $1 AND account_id = $2
//         "#,
//         token_id,
//         account_id
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record)
// }

// async fn insert_token_balance(pool: &Pool<Postgres>, balance: &TokenBalance) -> Result<i64, anyhow::Error> {
//     let record = sqlx::query!(
//         r#"
//         INSERT INTO token_balance (token_id, account_id, value, last_bought, last_sold, held_for)
//         VALUES ($1, $2, $3, $4, $5, $6)
//         RETURNING id
//         "#,
//         balance.token_id,
//         balance.account_id,
//         balance.value,
//         balance.last_bought,
//         balance.last_sold,
//         balance.held_for
//     )
//     .fetch_one(pool)
//     .await?;
    
//     Ok(record.id)
// }

// async fn update_token_balance_db(pool: &Pool<Postgres>, balance: &TokenBalance) -> Result<(), anyhow::Error> {
//     sqlx::query!(
//         r#"
//         UPDATE token_balances
//         SET value = $1, last_bought = $2, last_sold = $3, held_for = $4
//         WHERE id = $5
//         "#,
//         balance.value,
//         balance.last_bought,
//         balance.last_sold,
//         balance.held_for,
//         balance.id
//     )
//     .execute(pool)
//     .await?;
    
//     Ok(())
// }

// Event structs
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