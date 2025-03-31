use anyhow::Ok;
use anyhow::Result; 
use serde_json::Value;
use sqlx::{Transaction, Postgres, Executor}; 
use reqwest;
use chrono::{DateTime, TimeZone, Utc}; // For handling timestamps
use crate::models::{ Account, CultToken, TokenBalance, TokenTrade };
use sqlx::types::BigDecimal;
use std::str::FromStr;                  // for parsing string -> BigDecimal
use serde::Deserialize;
use serde::de::{self, Deserializer};

// Introduce 'tx for the Transaction's own lifetime, 'a for the borrow lifetime
pub async fn create_account<'tx, 'a>(
    // The transaction itself is valid for 'tx (tied to its connection)
    // We borrow it mutably for 'a within this function
    tx: &'a mut Transaction<'tx, Postgres>,
    address: &'a str,
    slug: Option<&'a str>,
    referrer_id: Option<&'a str>,
) -> Result<()>
where
    'tx: 'a, // Transaction must live at least as long as the borrow
{
    let referral_code: Option<String> = None;
    let diamond_hand_probability: i32 = 0;
    let total_referrals: i32 = 0;
    let fee_collected: f64 = 0.0;
    let twitter: Option<&str> = None;
    let discord: Option<&str> = None;

    sqlx::query(
        r#"
        INSERT INTO account (id, slug, referral_code, diamond_hand_probability, referrer_id, total_referrals, fee_collected, twitter, discord)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (id) DO NOTHING
        "#
    )
    .bind(address)
    .bind(slug)
    .bind(referral_code)
    .bind(diamond_hand_probability)
    .bind(referrer_id)
    .bind(total_referrals)
    .bind(fee_collected)
    .bind(twitter)
    .bind(discord)
    // Pass `tx` directly. `&'a mut Transaction<'tx, Pg>` implements Executor<'a, Pg>.
    .execute(&mut **tx).await?;

    Ok(())
}


pub async fn handle_cult_token_created(
    event: CultTokenCreatedEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    println!("handle_cult_token_created");
    // Convert u64 to i64 for SQLx compatibility
    let block_number = event.block_number.to_string();
    let block_timestamp = event.block_timestamp.to_string();

    let mut ipfs_content = "".to_string();
    // Handle IPFS data
    if event.token_uri.starts_with("ipfs://") {
        let hash = event.token_uri.trim_start_matches("ipfs://").to_string();
        let ipfs_url = format!("https://ipfs.io/ipfs/{}", hash);

        let client = reqwest::Client::new();
        match client.get(&ipfs_url).send().await {
            std::result::Result::Ok(response) => {
                if response.status().is_success() {
                     match response.text().await {
                         std::result::Result::Ok(text) => ipfs_content = text,
                         Err(e) => eprintln!("Failed to read IPFS text content: {}", e), // Log error, continue
                     }
                } else {
                    eprintln!("Failed IPFS fetch, status: {}", response.status()); // Log error, continue
                }
            }
            Err(e) => {
                eprintln!("Failed to send IPFS request: {}", e); // Log error, continue
            }
        }
    }

    // --- Type Conversions ---
    // Convert block_number to i64 for BIGINT compatibility
    let block_number_db: i64 = match event.block_number.try_into() {
        std::result::Result::Ok(bn) => bn,
        Err(_) => return Err(anyhow::anyhow!("Block number {} too large to fit in i64", event.block_number)),
   };

   // Convert u64 Unix timestamp to DateTime<Utc> for TIMESTAMPTZ
   let block_timestamp_db: DateTime<Utc> = Utc.timestamp_opt(event.block_timestamp as i64, 0).single()
       .ok_or_else(|| anyhow::anyhow!("Invalid block timestamp: {}", event.block_timestamp))?;

   // Numeric types - Using f64 here, consider BigDecimal if precision is paramount
   // and change DB columns to NUMERIC
   let price_db: f64 = 1_200_000_000_000.0;
   let market_cap_db: f64 = 6000.0;
   let circulating_supply_db: f64 = 0.0;
   let total_fee_db: f64 = 0.0;
   let volume_db: f64 = 0.0;

    // Insert the cult token
    sqlx::query(
        r#"
        INSERT INTO cult_token (
            id, factory_address, token_creator, protocol_fee_recipient, bonding_curve,
            token_uri, name, symbol, pool_address, block_number,
            block_timestamp, transaction_hash, holder_count, airdrop_contract, ipfs_content, chain, price, market_cap, circulating_supply, total_fee, volume
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
        "#
    )
    .bind(&event.token_address)
    .bind(&event.factory_address)
    .bind(&event.token_creator)
    .bind(&event.protocol_fee_recipient)
    .bind(&event.bonding_curve)
    .bind(&event.token_uri)
    .bind(&event.name)
    .bind(&event.symbol)
    .bind(&event.pool_address)
    .bind(block_number_db) // Use the converted block_number
    .bind(block_timestamp_db) // Use the converted block_timestamp
    .bind(&event.transaction_hash)
    .bind(0i64)
    .bind(&event.airdrop_contract)
    .bind(ipfs_content) 
    .bind(&event.chain_id) //chain -
    .bind(price_db) //price - assuming 0 for now (starting bonding curve price)
    .bind(market_cap_db) //market_cap - assuming 0 for now
    .bind(circulating_supply_db) //circulating_supply -needs to change based on airdrop contract
    .bind(total_fee_db)
    .bind(volume_db) //total_fee - assuming 0 for now
    .execute(&mut **tx).await?;

    // Create accounts
    create_account(&mut *tx, &event.pool_address, Some("POOL"), None).await?;
    create_account(&mut *tx, &event.token_creator, Some("CREATOR"), None).await?;
    create_account(&mut *tx, &event.airdrop_contract, Some("AIRDROP"), None).await?;

    // Initialize token_stats
    sqlx::query(
        r#"
        INSERT INTO token_stats (
            token_id, mean_duration, stddev_duration, mean_pnl, stddev_pnl,
            mean_value, stddev_value, mean_volume, stddev_volume, liquidity_score, holder_weight
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#
    )
    .bind(&event.token_address)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .bind(0.0f64)
    .execute(&mut **tx).await?;
    
    Ok(())
}

pub async fn handle_cult_token_buy(
    event: CultTokenBuyEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    println!("handle_cult_token_buy");
    // Convert u128 values to string (already done), then parse as BigDecimal:
    let total_eth_bd = BigDecimal::from_str(&event.total_eth.to_string())?;
    let eth_fee_bd = BigDecimal::from_str(&event.eth_fee.to_string())?;
    let eth_sold_bd = BigDecimal::from_str(&event.eth_sold.to_string())?;
    let tokens_bought_bd = BigDecimal::from_str(&event.tokens_bought.to_string())?;
    let buyer_token_balance_bd = BigDecimal::from_str(&event.buyer_token_balance.to_string())?;
    let total_supply_bd = BigDecimal::from_str(&event.total_supply.to_string())?;

    // Create accounts if they don't exist
    create_account(&mut *tx, &event.trader_id, None, None).await?;
    create_account(&mut *tx, &event.order_referrer, None, None).await?;

    // 1) Insert trade record
    sqlx::query!(
        r#"
        INSERT INTO token_trade (
            token_id,
            trade_type,
            trader_id,
            recipient_id,
            order_referrer_id,
            total_eth,
            eth_fee,
            eth_amount,
            token_amount,
            trader_token_balance,
            total_supply,
            market_type,
            timestamp,
            transaction_hash
        )
        VALUES (
            $1, 'Buy'::trade_type, $2, $3, $4,
            $5, $6, $7, $8,
            $9, $10, $11,
            TO_TIMESTAMP($12), $13
        )
        ON CONFLICT (transaction_hash, token_id) DO NOTHING
        RETURNING token_id
        "#,
        event.token_id,
        event.trader_id,
        event.trader_id, //recipient and trader are same by default for now
        event.order_referrer,
        total_eth_bd,           // BigDecimal for NUMERIC
        eth_fee_bd,
        eth_sold_bd,
        tokens_bought_bd,
        buyer_token_balance_bd,
        total_supply_bd,
        event.market_type as i16,
        event.block_timestamp as i64,
        event.transaction_hash
    )
    .fetch_optional(&mut **tx)
    .await?;

    // 2) Calculate price & market_cap, then parse them as BigDecimal
    let price_str = format!("{:.18}", event.eth_sold as f64 / event.tokens_bought as f64);
    let market_cap_str = (event.eth_sold as f64 * event.total_supply as f64 / event.tokens_bought as f64).to_string();

    let price_bd = BigDecimal::from_str(&price_str)?;
    let market_cap_bd = BigDecimal::from_str(&market_cap_str)?;

    // 3) Insert/Update the user's token_balance
    sqlx::query!(
        r#"
        INSERT INTO token_balance (
            account_id,
            token_id,
            first_bought,
            volume,
            holding_duration,
            pnl,
            holdings_value,
            duration_z,
            pnl_z,
            value_z
        )
        VALUES (
            $1,
            $2,
            NOW(),
            $3,
            0,
            0,
            $4,
            0,
            0,
            0
        )
        ON CONFLICT (account_id, token_id)
        DO UPDATE SET
            volume = token_balance.volume + EXCLUDED.volume,
            holdings_value = EXCLUDED.holdings_value
        "#,
        event.trader_id,
        event.token_id,
        total_eth_bd,                // volume = total_eth as BigDecimal
        buyer_token_balance_bd       // holdings_value
    )
    .execute(&mut **tx)
    .await?;

    // 4) Update cult_token
    sqlx::query!(
        r#"
        WITH new_holder AS (
            SELECT NOT EXISTS (
                SELECT 1 FROM token_balance
                WHERE token_id = $1 AND account_id = $2 AND holdings_value > 0
            ) AS is_new_holder
        )
        UPDATE cult_token
        SET
            holder_count = holder_count + 
                           CASE WHEN (SELECT is_new_holder FROM new_holder) THEN 1 ELSE 0 END,
            price = $3,
            circulating_supply = $4,
            market_cap = $5,
            total_fee = total_fee + $6,
            volume = volume + $7
        WHERE id = $1
        "#,
        event.token_id,
        event.trader_id,
        price_bd,
        total_supply_bd,
        market_cap_bd,
        eth_fee_bd,
        total_eth_bd
    )
    .execute(&mut **tx)
    .await?;

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
//         transaction_hash: event.transaction_hash.clone(),
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
//         transaction_hash: event.transaction_hash.clone(),
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
//         transaction_hash: event.transaction_hash.clone(),
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

fn deserialize_u128_from_str<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    u128::from_str(&s).map_err(de::Error::custom)
}

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
    pub transaction_hash: String,
    pub chain_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CultTokenBuyEvent {
    pub trader_id: String,             
    pub recipient_id: String,          
    pub order_referrer: String,       
    #[serde(deserialize_with = "deserialize_u128_from_str")] 
    pub total_eth: u128,               
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub eth_fee: u128,                 
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub eth_sold: u128,                
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub tokens_bought: u128,           
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub buyer_token_balance: u128,     
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub total_supply: u128,            
    pub market_type: u8,               
    pub block_timestamp: u64,          
    pub block_number: u64,             
    pub transaction_hash: String,      
    pub token_id: String,              
    pub chain_id: u64                  
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
    pub transaction_hash: String,
}

#[derive(serde::Deserialize)]
pub struct CultTokenTransferEvent {
    pub from: String,
    pub to: String,
    pub from_token_balance: i64,
    pub to_token_balance: i64,
    pub block_timestamp: u64,
    pub transaction_hash: String,
}

#[derive(serde::Deserialize)]
pub struct CultTokenFeesEvent {
    pub order_referrer: String,
    pub order_referrer_fee: i64,
    pub block_timestamp: u64,
    pub transaction_hash: String,
}
