use alloy::primitives::U256;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;
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
    pub pool_address: String,
    pub block_number: i64,
    pub block_timestamp: DateTime<Utc>,
    pub transaction_hash: String,
    pub holder_count: u32,
    pub airdrop_contract: String,
    pub trades: Option<Vec<TokenTrade>>,
    pub balances: Option<Vec<TokenBalance>>,
    pub ipfs_content: String,
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

#[derive(Debug)]
struct TokenMetrics {
    token_id: String,
    mean_duration: f64,
    stddev_duration: f64,
    mean_pnl: f64,
    stddev_pnl: f64,
    mean_value: f64,
    stddev_value: f64,
}
// Configuration struct for webhook settings
#[derive(Clone)]
pub struct WebhookConfig {
    pub secret_key: String,
    pub max_concurrent_jobs: usize,
    pub job_queue_buffer: usize,
}

// Webhook payload structures
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WebhookEventType {
    CultTokenCreated,
    CultTokenBuy,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event_type: WebhookEventType,
    pub data: serde_json::Value,
    pub timestamp: i64,
}

// Response for webhook receipt
#[derive(Serialize)]
pub struct WebhookResponse {
    pub status: String,
    pub event_id: String,
}

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