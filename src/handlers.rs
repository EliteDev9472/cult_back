use anyhow::Ok;
use anyhow::Result; 

use sqlx::{Transaction, Postgres}; 
use reqwest;
use chrono::{DateTime, TimeZone, Utc}; // For handling timestamps
use sqlx::types::BigDecimal;
use std::str::FromStr;                  // for parsing string -> BigDecimal
use serde::Deserialize;
use serde::de::{self, Deserializer};
use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

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

   let total_airdrop_recipient_count_db: i64 = match event.total_airdrop_recipient_count.try_into() {
    std::result::Result::Ok(ta) => ta,
    Err(_) => return Err(anyhow::anyhow!("total_airdrop_recipient_count {} too large to fit in i64", event.total_airdrop_recipient_count)),
};

let total_amount = BigDecimal::from_str(&event.total_amount.to_string())?;

   // Convert u64 Unix timestamp to DateTime<Utc> for TIMESTAMPTZ
   let block_timestamp_db: DateTime<Utc> = Utc.timestamp_opt(event.block_timestamp as i64, 0).single()
       .ok_or_else(|| anyhow::anyhow!("Invalid block timestamp: {}", event.block_timestamp))?;

   // Numeric types - Using f64 here, consider BigDecimal if precision is paramount
   // and change DB columns to NUMERIC
   let price_db: f64 = 1_200_000_000_000.0; //based on A value in bonding vurve
   let market_cap_db: f64 = 6000.0; // based on
   let total_fee_db: f64 = 0.0;
   let volume_db: f64 = 0.0;

   let top_holders = &total_amount/BigDecimal::from_str("10_000_000_000")?; // airdropped_amount / total supply
 let bonding_curve_percentage = 0.0;
    // Insert the cult token
    sqlx::query(
        r#"
        INSERT INTO cult_token (
            id, factory_address, token_creator, protocol_fee_recipient, bonding_curve,
            token_uri, name, symbol, pool_address, block_number,
            block_timestamp, transaction_hash, holder_count, airdrop_contract, ipfs_content, chain, 
            price, market_cap, circulating_supply, total_fee, volume,total_airdrop_recipient_count,total_amount,
top_holders,bonding_curve_percentage
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25)
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
    .bind(total_airdrop_recipient_count_db) //initial holder count
    .bind(&event.airdrop_contract)
    .bind(ipfs_content) 
    .bind("10143") //chain -
    .bind(price_db) //price - assuming 0 for now (starting bonding curve price)
    .bind(market_cap_db) //market_cap - assuming 0 for now
    .bind(&total_amount) //circulating_supply currently based on airdrop amount
    .bind(total_fee_db)
    .bind(volume_db)
    .bind(total_airdrop_recipient_count_db)
    .bind(&total_amount)
    
    .bind(&top_holders)
    .bind(&bonding_curve_percentage)
     //total_fee - assuming 0 for now
    .execute(&mut **tx).await?;

    // Create accounts
    create_account(&mut *tx, &event.pool_address, Some("POOL"), None).await?;
    // not needed as creators will always have account for now
    //create_account(&mut *tx, &event.token_creator, Some("CREATOR"), None).await?;
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
    

    // Process each merkle root from the event
    for merkle_root in event.merkle_roots {
        
        // Convert hex string (with or without 0x prefix) to Vec<u8>
        let merkle_root_bytes = hex::decode(merkle_root.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid merkle root hex: {}, error: {}", merkle_root, e))?;
    
        
        // Find the community for this merkle root
        let community: Option<(String, String, serde_json::Value)> = sqlx::query_as(
            r#"
            SELECT id, name, merkle_proofs 
            FROM communities 
            WHERE merkle_root = $1
            "#
        )
        .bind(&merkle_root_bytes)
        .fetch_optional(&mut **tx)
        .await?;

        let (community_id, community_name, merkle_proofs) = match community {
            Some(c) => c,
            None => {
                eprintln!("No community found for merkle root: {:?}", merkle_root);
                continue; // Skip but continue processing other roots
            }
        };

        // Convert recipient count to i64
        let recipients_db: i64 = event.total_airdrop_recipient_count
            .try_into()
            .map_err(|_| anyhow::anyhow!("Recipient count exceeds i64 bounds"))?;

        // Insert into token_airdrops using event totals directly
        sqlx::query(
            r#"
            INSERT INTO token_airdrops (
                transaction_hash, 
                token_id, 
                merkle_root, 
                community_id, 
                community_name, 
                merkle_proofs, 
                total_amount, 
                total_recipient_count, 
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            "#
        )
        .bind(&event.transaction_hash)
        .bind(&event.token_address)
        .bind(&merkle_root_bytes)
        .bind(&community_id)
        .bind(&community_name)
        .bind(&merkle_proofs)
        .bind(&total_amount) // Use total directly from event
        .bind(recipients_db)       // Use total directly from event
        .execute(&mut **tx)
        .await?;

        let merkle_proofs_obj = merkle_proofs
        .as_object()
        .ok_or(anyhow::anyhow!("Invalid merkle_proofs format"))?;
    


        let recipient_addresses: Vec<&str> = merkle_proofs_obj.keys().map(|k| k.as_str()).collect();


        sqlx::query(
            r#"
            WITH new_airdrop AS (
                SELECT currval('token_airdrops_id_seq') AS id
            )
            INSERT INTO airdrop_recipients (account_id, token_airdrop_id)
            SELECT unnest($1::TEXT[]), na.id
            FROM new_airdrop na
            ON CONFLICT DO NOTHING
            "#
        )
        .bind(&recipient_addresses)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}


//to update token balance, we should probably look at thoken transfer
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
    //atm only existing traders can trade on our platform so commenting this
    //create_account(&mut *tx, &event.trader_id, None, None).await?;
    //create_account(&mut *tx, &event.order_referrer, None, None).await?;

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

pub async fn handle_cult_token_sell(
    event: CultTokenSellEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    println!("handle_cult_token_sell");

    // Convert values to BigDecimal
    let total_eth_bd = BigDecimal::from_str(&event.total_eth.to_string())?;
    let eth_fee_bd = BigDecimal::from_str(&event.eth_fee.to_string())?;
    let eth_bought_bd = BigDecimal::from_str(&event.eth_bought.to_string())?;
    let tokens_sold_bd = BigDecimal::from_str(&event.tokens_sold.to_string())?;
    let seller_token_balance_bd = BigDecimal::from_str(&event.seller_token_balance.to_string())?;
    let total_supply_bd = BigDecimal::from_str(&event.total_supply.to_string())?;

    // Create accounts if they don't exist
    //atm only existing traders can trade on our platform so commenting this
    //create_account(&mut *tx, &event.seller, None, None).await?;
    //create_account(&mut *tx, &event.recipient, None, None).await?;
    //create_account(&mut *tx, &event.order_referrer, None, None).await?;

    // Insert trade record
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
            $1, 'Sell'::trade_type, $2, $3, $4,
            $5, $6, $7, $8,
            $9, $10, $11,
            TO_TIMESTAMP($12), $13
        )
        ON CONFLICT (transaction_hash, token_id) DO NOTHING
        RETURNING token_id
        "#,
        event.token_id,
        event.seller,
        event.recipient,
        event.order_referrer,
        total_eth_bd,
        eth_fee_bd,
        eth_bought_bd,
        tokens_sold_bd,
        seller_token_balance_bd,
        total_supply_bd,
        event.market_type as i16,
        event.block_timestamp as i64,
        event.transaction_hash
    )
    .fetch_optional(&mut **tx)
    .await?;

    // 2) Calculate price & market_cap, then parse them as BigDecimal
    let price_str = format!("{:.18}", event.eth_bought as f64 / event.tokens_sold as f64);
    let market_cap_str = (event.eth_bought as f64 * event.total_supply as f64 / event.tokens_sold as f64).to_string();

    let price_bd = BigDecimal::from_str(&price_str)?;
    let market_cap_bd = BigDecimal::from_str(&market_cap_str)?;

    // 3) Insert/Update the user's token_balance
    // Combined query that updates holding_duration only if it's NULL or 0
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
            holdings_value = EXCLUDED.holdings_value,
            holding_duration = CASE 
                WHEN token_balance.holding_duration IS NULL OR token_balance.holding_duration = 0 
                THEN EXTRACT(EPOCH FROM (NOW() - token_balance.first_bought))
                ELSE token_balance.holding_duration
            END
        "#,
        event.seller,
        event.token_id,
        total_eth_bd,
        seller_token_balance_bd
    )
    .execute(&mut **tx)
    .await?;

    let should_decrement = seller_token_balance_bd == BigDecimal::from(0);

    // 4) Update cult_token
    // Use different queries based on the condition
    let holder_count_adjustment = if should_decrement { -1 } else { 0 };

    sqlx::query!(
        r#"
        UPDATE cult_token
        SET
            holder_count = holder_count + $7,
            price = $2,
            circulating_supply = $3,
            market_cap = $4,
            total_fee = total_fee + $5,
            volume = volume + $6
        WHERE id = $1
        "#,
        event.token_id,
        price_bd,
        total_supply_bd,
        market_cap_bd,
        eth_fee_bd,
        total_eth_bd,
        holder_count_adjustment
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn handle_cult_token_transfer(
    event: CultTokenTransferEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    println!("TRANSFERING TOKEN");

    // Skip zero address transfers
    if event.from == "0x0000000000000000000000000000000000000000" {
        return Ok(());
    }

    // Create accounts if they don't exist
  //atm only existing traders can trade on our platform so commenting this  
    //create_account(&mut *tx, &event.from, None, None).await?;
    //create_account(&mut *tx, &event.to, None, None).await?;

    // Convert balances to BigDecimal
    let from_balance_bd = BigDecimal::from_str(&event.from_token_balance.to_string())?;
    let to_balance_bd = BigDecimal::from_str(&event.to_token_balance.to_string())?;

    // Update balances for both accounts
    // First update the sender's balance
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
            0,
            0,
            0,
            $3,
            0,
            0,
            0
        )
        ON CONFLICT (account_id, token_id)
        DO UPDATE SET
            holdings_value = EXCLUDED.holdings_value
        "#,
        event.from,
        event.token_id,
        from_balance_bd
    )
    .execute(&mut **tx)
    .await?;

    // Then update the receiver's balance
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
            0,
            0,
            0,
            $3,
            0,
            0,
            0
        )
        ON CONFLICT (account_id, token_id)
        DO UPDATE SET
            holdings_value = EXCLUDED.holdings_value,
            first_bought = CASE 
                WHEN token_balance.first_bought IS NULL THEN NOW()
                ELSE token_balance.first_bought
            END
        "#,
        event.to,
        event.token_id,
        to_balance_bd
    )
    .execute(&mut **tx)
    .await?;

    // Update holder count in cult_token
    // We need to check if:
    // 1. Sender's balance went to 0 (decrement count)
    // 2. Receiver's balance went from 0 to >0 (increment count)
    sqlx::query!(
        r#"
        WITH balance_changes AS (
            SELECT 
                -- Check if sender is losing all tokens
                (SELECT holdings_value = 0 FROM token_balance 
                 WHERE account_id = $1 AND token_id = $2) AS sender_empty,
                
                -- Check if receiver is getting first tokens
                (SELECT holdings_value > 0 FROM token_balance 
                 WHERE account_id = $3 AND token_id = $2) AS receiver_had_tokens
        )
        UPDATE cult_token
        SET
            holder_count = holder_count + 
                          CASE 
                              WHEN (SELECT sender_empty FROM balance_changes) AND 
                                   (SELECT NOT receiver_had_tokens FROM balance_changes) 
                              THEN -1
                              WHEN (SELECT NOT sender_empty FROM balance_changes) AND 
                                   (SELECT NOT receiver_had_tokens FROM balance_changes) 
                              THEN 1
                              WHEN (SELECT sender_empty FROM balance_changes) AND 
                                   (SELECT receiver_had_tokens FROM balance_changes) 
                              THEN 0
                              ELSE 0
                          END
        WHERE id = $2
        "#,
        event.from,
        event.token_id,
        event.to
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn handle_cult_token_fees(
    event: CultTokenFeesEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    println!("CultTokenFees: {}", event.order_referrer);

    let fee_bd = BigDecimal::from_str(&event.order_referrer_fee.to_string())?;

    // Get existing account
    let existing_account = sqlx::query!(
        r#"
        SELECT 
            slug,
            referrer_id,
            total_referrals,
            fee_collected,
            diamond_hand_probability
        FROM account 
        WHERE id = $1
        "#,
        event.order_referrer
    )
    .fetch_optional(&mut **tx)
    .await?;

    // Update or create account with accumulated fees
    sqlx::query!(
        r#"
        INSERT INTO account (
            id,
            slug,
            referrer_id,
            total_referrals,
            fee_collected,
            diamond_hand_probability
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id) DO UPDATE SET
            fee_collected = COALESCE(account.fee_collected, 0) + $5::numeric,
            total_referrals = COALESCE(account.total_referrals, 0) + 1,
            slug = COALESCE(account.slug, $2),
            diamond_hand_probability = COALESCE(account.diamond_hand_probability, $6)
        "#,
        event.order_referrer,
        existing_account.as_ref().and_then(|a| a.slug.clone())
            .unwrap_or_else(|| "Order Referrer Fees".to_string()),
        existing_account.as_ref().and_then(|a| a.referrer_id.clone()),
        existing_account.as_ref().map(|a| a.total_referrals.unwrap_or(0) + 1).unwrap_or(1),
        fee_bd,
        existing_account.as_ref().map(|a| a.diamond_hand_probability).unwrap_or(0)
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn update_token_balance(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    token_id: &str,
    account_id: &str,
    new_value: u128,
    timestamp: u64
) -> Result<(), anyhow::Error> {
    println!("UPDATING TOKEN BALANCE");

    let current_timestamp = Utc.timestamp_opt(timestamp as i64, 0).unwrap();
    let zero_decimal = BigDecimal::from_str("0").unwrap();

    // Get existing balance
    let existing = sqlx::query!(
        r#"
        SELECT holdings_value, first_bought FROM token_balance 
        WHERE account_id = $1 AND token_id = $2
        "#,
        account_id,
        token_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    let old_value = existing.map(|b| b.holdings_value).unwrap_or(zero_decimal.clone());

    // Insert or update balance - Using composite key for ON CONFLICT
    sqlx::query!(
        r#"
        INSERT INTO token_balance (
            token_id,
            account_id,
            holdings_value,
            first_bought,
            holding_duration,
            pnl,
            duration_z,
            pnl_z,
            value_z
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (account_id, token_id) DO UPDATE SET
            holdings_value = $3,
            holding_duration = EXTRACT(EPOCH FROM (NOW() - token_balance.first_bought))::bigint
        "#,
        token_id,
        account_id,
        BigDecimal::from(new_value),
        current_timestamp,
        0i64.into(),  // holding_duration
        Some(BigDecimal::from(0)),  // pnl
        Some(BigDecimal::from_str("0.0").unwrap()),  // duration_z
        Some(BigDecimal::from_str("0.0").unwrap()),  // pnl_z
        Some(BigDecimal::from_str("0.0").unwrap())   // value_z
    )
    .execute(&mut **tx)
    .await?;

    // Update holder count
    if (old_value == zero_decimal && new_value > 0) || (old_value > zero_decimal && new_value == 0) {
        sqlx::query!(
            r#"
            UPDATE cult_token
            SET holder_count = (
                SELECT COUNT(DISTINCT account_id)
                FROM token_balance
                WHERE token_id = $1 AND holdings_value > 0
            )
            WHERE id = $1
            "#,
            token_id
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

pub async fn handle_token_claimed(
    event: TokensClaimedEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    println!("handle_token_claimed");

    let token = event.token.to_string();
    let recipient = event.recipient.to_string();
    let amount = BigDecimal::from_str(&event.amount.to_string())?;

    // Check if the recipient is already holding the token
    let is_new_holder: Option<bool> = sqlx::query_scalar!(
        r#"
        SELECT NOT EXISTS (
            SELECT 1 
            FROM token_balance
            WHERE account_id = $1 AND token_id = $2 AND holdings_value > 0
        ) AS is_new_holder
        "#,
        recipient,
        token
    )
    .fetch_optional(&mut **tx)
    .await?
    .flatten(); // Flatten the Option<Option<bool>> to Option<bool>

    let is_new_holder = is_new_holder.unwrap_or(false);

    // Update the token_balance for the recipient
    sqlx::query!(
        r#"
        UPDATE token_balance
        SET 
            holdings_value = holdings_value + $3,
            first_bought = CASE 
                WHEN $4 THEN CURRENT_TIMESTAMP
                ELSE first_bought
            END
        WHERE account_id = $1 AND token_id = $2;
        "#,
        recipient,  // account_id
        token,      // token_id
        amount,     // amount
        is_new_holder // whether the recipient is a new holder
    )
    .execute(&mut **tx)
    .await?;

    // Update holder_count in cult_token if the recipient is a new holder
    if is_new_holder {
        sqlx::query!(
            r#"
            UPDATE cult_token
            SET holder_count = holder_count + 1
            WHERE id = $1;
            "#,
            token
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

pub async fn handle_cult_market_graduated(
    event: CultMarketGraduatedEvent,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), anyhow::Error> {
    println!("handle_cult_market_graduated");
    // Convert u128 values to string (already done), then parse as BigDecimal:
    let tokenAddress = event.tokenAddress.to_string();
    let poolAddress = event.poolAddress.to_string();
    let totalEthLiquidity = BigDecimal::from_str(&event.totalEthLiquidity.to_string())?;
    let totalTokenLiquidity = BigDecimal::from_str(&event.totalTokenLiquidity.to_string())?;
    let lpPositionId = i64::from_str(&event.lpPositionId.to_string())?;
    let marketType = u8::from_str(&event.marketType.to_string())?;

    // update token_balance
    sqlx::query!(
        r#"
        update cult_token
        set lppositionId = $3,
        is_graduated = $4
        where id = $1 and pool_address = $2
        "#,
        tokenAddress,
        poolAddress,
        lpPositionId,
        marketType == 1
    )
    .fetch_optional(&mut **tx)
    .await?;

    Ok(())
}

pub async fn start_price_fetcher(pool: PgPool) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = fetch_and_store_prices(&pool).await {
                eprintln!("Price fetch error: {:?}", e);
            }
            sleep(Duration::from_secs(2)).await;
        }
    });
}

async fn fetch_and_store_prices(pool: &PgPool) -> Result<(), anyhow::Error> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum&vs_currencies=usd&include_24hr_change=true&include_1hr_change=true";
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    let data: HashMap<String, PriceEntry> = response.json().await?;

    println!("--------fetch_and_store_prices-----------");
    for (symbol, entry) in data {
        let symbol_upper = match symbol.as_str() {
            "bitcoin" => "BTC",
            "ethereum" => "ETH",
            _ => &symbol.to_uppercase()
        };

        sqlx::query!(
            r#"
            INSERT INTO crypto_latest_prices (symbol, price, change_24h, change_1h, last_updated_at)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)
            ON CONFLICT (symbol)
            DO UPDATE SET
                price = EXCLUDED.price,
                change_24h = EXCLUDED.change_24h,
                change_1h = EXCLUDED.change_1h,
                last_updated_at = EXCLUDED.last_updated_at
            "#,
            symbol_upper,
            entry.usd,
            entry.usd_24h_change,
            entry.usd_1h_change
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn deserialize_u128_from_str<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where D: Deserializer<'de>
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
    pub merkle_roots: Vec<String>,      
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub total_amount: u128,         
    pub total_airdrop_recipient_count: u32,
}

#[derive(Debug, serde::Deserialize, Clone, serde::Serialize)]
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
    pub chain_id: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct CultTokenSellEvent {
    pub seller: String,
    pub recipient: String,
    pub order_referrer: String,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub total_eth: u128,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub eth_fee: u128,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub eth_bought: u128,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub tokens_sold: u128,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub seller_token_balance: u128,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub total_supply: u128,
    pub market_type: u8,
    pub block_timestamp: u64,
    pub block_number: u64,
    pub log_index: u64,
    pub chain_id: u64,
    pub transaction_hash: String,
    pub token_id: String,
}

// Update CultTokenTransferEvent struct to include token_id
#[derive(serde::Deserialize)]
pub struct CultTokenTransferEvent {
    pub from: String,
    pub to: String,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub from_token_balance: u128,
    #[serde(deserialize_with = "deserialize_u128_from_str")]
    pub to_token_balance: u128,
    pub block_timestamp: u64,
    pub transaction_hash: String,
    pub token_id: String,
}

#[derive(serde::Deserialize)]
pub struct CultTokenFeesEvent {
    pub order_referrer: String,
    pub order_referrer_fee: i64,
    pub block_timestamp: u64,
    pub transaction_hash: String,
}

#[derive(serde::Deserialize)]
pub struct TokensClaimedEvent {
    pub token: String,
    pub recipient: String,
    pub amount: BigDecimal
}

#[derive(serde::Deserialize)]
pub struct CultMarketGraduatedEvent {
    pub tokenAddress: String,
    pub poolAddress: String,
    pub totalEthLiquidity: BigDecimal,
    pub totalTokenLiquidity: BigDecimal,
    pub lpPositionId: i64,
    pub marketType: u8
}

#[derive(Debug, Deserialize)]
pub struct PriceEntry {
    usd: BigDecimal,
    usd_24h_change: BigDecimal,
    usd_1h_change: BigDecimal,
}