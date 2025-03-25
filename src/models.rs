use alloy::primitives::U256;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CultToken {
    pub id: Option<String>, // Changed to String for address
    pub factory_address: String,
    pub token_creator: String,
    pub protocol_fee_recipient: String,
    pub bonding_curve: String,
    pub token_uri: String,
    pub name: String,
    pub symbol: String,
    pub token_address: String,
    pub pool_address: String,
    pub block_number: i64,
    pub block_timestamp: DateTime<Utc>,
    pub transaction_hash: String,
    pub holder_count: i64,
    pub airdrop_contract: String,
    pub trades: Option<Vec<TokenTrade>>,
    pub balances: Option<Vec<TokenBalance>>,
    pub ipfs_data: Option<Vec<TokenIPFSData>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TokenIPFSData {
    pub id: Option<String>, // Changed to String for concatenated hash and address
    pub hash: String,
    pub content: String,
    pub token_id: String, // Reference to CultToken (foreign key)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TradeType {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub id: Option<String>, // Changed to String for address
    pub slug: Option<String>,
    pub referral_code: Option<String>,
    pub diamond_hand_probability: u32,
    pub referrer_id: Option<String>, // Reference to another Account (foreign key)
    pub total_referrals: Option<u32>,
    pub fee_collected: U256,
    pub twitter: Option<String>,
    pub discord: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TokenBalance {
    pub id: Option<String>, // Changed to String for concatenated hash and address
    pub token_id: String, // Reference to CultToken (foreign key)
    pub account_id: String, // Reference to Account (foreign key)
    pub value: i64,
    pub last_bought: DateTime<Utc>,
    pub last_sold: DateTime<Utc>,
    pub held_for: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TokenTrade {
    pub id: Option<String>, // Changed to String for concatenated hash and address
    pub token_id: String, // Reference to CultToken (foreign key)
    pub trade_type: TradeType,
    pub trader_id: String, // Reference to Account (foreign key)
    pub recipient_id: String, // Reference to Account (foreign key)
    pub order_referrer_id: String, // Reference to Account (foreign key)
    pub total_eth: i64,
    pub eth_fee: i64,
    pub eth_amount: i64,
    pub token_amount: i64,
    pub trader_token_balance: i64,
    pub total_supply: i64,
    pub market_type: i64,
    pub timestamp: DateTime<Utc>,
    pub transaction_hash: String,
}