use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use bigdecimal::BigDecimal;

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct CultToken {
    pub id: Option<String>,
    pub factory_address: String,
    pub token_creator: String,
    pub protocol_fee_recipient: String,
    pub bonding_curve: String,
    pub token_uri: String,
    pub name: String,
    pub symbol: String,
    //pub token_address: String,
    pub pool_address: String,
    pub block_number: i64,
    pub block_timestamp: chrono::DateTime<chrono::Utc>,
    pub transaction_hash: String,
    pub holder_count: i64,
    pub airdrop_contract: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TokenIPFSData {
    pub id: Option<String>, // Changed to String for concatenated hash and address
    pub hash: String,
    pub content: String,
    pub token_id: String, // Reference to CultToken (foreign key)
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "trade_type")] // must match the SQL enum name exactly (case-sensitive)
pub enum TradeType {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub id: Option<String>,
    pub slug: Option<String>,
    pub referral_code: Option<String>,
    pub diamond_hand_probability: i32,
    pub referrer_id: Option<String>,
    pub total_referrals: Option<i32>,
    pub fee_collected: i64,  // 💡 store as string, or use Decimal
    pub twitter: Option<String>,
    pub discord: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TokenBalance {
    pub account_id: String, // Primary key
    pub token_id: String, // Reference to CultToken (foreign key)
    pub first_bought: DateTime<Utc>,
    pub volume: i64,
    pub holding_duration: i64,
    pub pnl: i64,
    pub holdings_value: i64,
    pub duration_z: i64,
    pub pnl_z: i64,
    pub value_z: i64,
}

// #[derive(Debug, Serialize, Deserialize, FromRow)]
// pub struct TokenTrade {
//     pub id: Option<String>, // Changed to String for concatenated hash and address
//     pub token_id: String, // Reference to CultToken (foreign key)
//     pub trade_type: TradeType,
//     pub trader_id: String, // Reference to Account (foreign key)
//     pub recipient_id: String, // Reference to Account (foreign key)
//     pub order_referrer_id: String, // Reference to Account (foreign key)
//     pub total_eth: i64,
//     pub eth_fee: i64,
//     pub eth_amount: i64,
//     pub token_amount: i64,
//     pub trader_token_balance: i64,
//     pub total_supply: i64,
//     pub market_type: i64,
//     pub timestamp: DateTime<Utc>,
//     pub transaction_hash: String,
// }


#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenTradeRow {
    pub token_id: String,
    pub trade_type: TradeType,
    pub trader_id: String,
    pub recipient_id: String,
    pub order_referrer_id: String,
    pub total_eth: BigDecimal,
    pub eth_fee: BigDecimal,
    pub eth_amount: BigDecimal,
    pub token_amount: BigDecimal,
    pub trader_token_balance: BigDecimal,
    pub total_supply: BigDecimal,
    // pub market_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub transaction_hash: String,
}

#[derive(Deserialize, Debug)]
pub struct AccountRef {
    pub id: String,
}

#[derive(Deserialize, Debug)]
pub struct CultTokensResponse {
    pub cult_tokens: Vec<CultToken>,
}

#[derive(Deserialize, Debug)]
pub struct TopHoldersResponse {
    pub token_balances: Vec<TokenBalance>,
}


#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct TopHolderParams {
    pub token_address: String,
    pub offset: i64,
    pub limit: i64,
}