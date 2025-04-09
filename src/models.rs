//use alloy::primitives::u128;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use bigdecimal::BigDecimal;
use utoipa::{OpenApi, ToSchema};

//////////////
// WEBHOOKS //
//////////////
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
    TokenClaimed,
    CultMarketGraduated
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

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Account {
    pub id: Option<String>, // Changed to String for address
    pub slug: Option<String>,
    pub referral_code: Option<String>,
    pub diamond_hand_probability: u32,
    pub referrer_id: Option<String>, // Reference to another Account (foreign key)
    pub total_referrals: Option<u32>,
    pub fee_collected: u128,
    pub twitter: Option<String>,
    pub discord: Option<String>,
    pub tokens_created: u32,
    pub tokens_migrated: u32
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


#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams)]
pub struct PaginationParams {
    pub offset: i64,
    pub limit: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    pub user_id: String,
    pub referral_code: Option<String>,
    pub twitter: Option<String>,
    pub discord: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateAccountResponse {
    pub user_id: String,
    pub referral_code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WatchlistActionRequest {
    pub account_id: String,
    pub cult_token_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAccountRequest {
    pub twitter: Option<String>,
    pub discord: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommunityResponse {
    pub name: String,
    pub img_url: String,
    pub chain: String,
    pub merkle_root: String,
    pub holder_count: i64,
    pub community_score: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct TopHolderParams {
    pub token_address: String,
    pub offset: i64,
    pub limit: i64,
}
#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CultTokensResponse {
    pub token_address: String,
    pub token_creator: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_data: String,
    pub holder_count: u32,
    pub market_cap: f64,
    pub volume: f64,
    pub total_airdrop_recipient_count: u32,
    pub creator_holdings: f64,
    pub top_holders: f64,
    pub buy_tx_count_1h: u32,
    pub sell_tx_count_1h: u32,
    #[schema(value_type = Option<String>, example = "2023-01-01T00:00:00Z")]
    pub last_traded: Option<DateTime<Utc>>,  // Changed to Option
    pub bonding_curve_percentage: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CultTokenTopHolders {
    pub value: BigDecimal,
    pub id: String
}



#[derive(Debug, Serialize, ToSchema)]
pub struct CultTokenDataResponse {
    pub id: String,
    pub token_creator: String,
    pub name: String,
    pub symbol: String,
    pub ipfs_content: String,
    pub holder_count: u32,
    pub market_cap: f64,
    pub volume: f64,
    pub total_airdrop_recipient_count: u32,
    pub creator_holdings: f64,
    pub top_holders: f64,
    pub buy_tx_count_1h: u32,
    pub sell_tx_count_1h: u32,
    #[schema(value_type = Option<String>, example = "2023-01-01T00:00:00Z")]
    pub last_traded: Option<DateTime<Utc>>, 
    pub bonding_curve_percentage: f64,
    pub is_graduated: bool,
    pub pool_address: String,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct TokenTradesResponse {
    pub id: String,
    pub trader: Option<String>,
    pub recipient: Option<String>,
    pub orderReferrer: Option<String>,
    pub ethAmount: Option<BigDecimal>,
    pub tokenAmount: Option<BigDecimal>,
    pub traderTokenBalance: Option<BigDecimal>,
    pub marketType: i64,
    pub timestamp: DateTime<Utc>,
    pub transactionHash: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountDetailResponse {
    pub id: String,
    pub slug: Option<String>,
    pub diamond_hand_probability: Option<i32>, // ✅ changed
    pub total_referrals: Option<i32>,          // ✅ changed
    pub feeCollected: Option<BigDecimal>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountData {
   pub created_tokens: Vec<CreatedToken>,
   pub owned_tokens: Vec<OwnedToken>,
   pub watchlist: Vec<WatchlistToken>,
   pub communities: Vec<Community>,
}

#[derive(Debug, Serialize, Deserialize,ToSchema)]
pub struct CreatedToken {
   pub id: String,
   pub name: String,
   pub symbol: String,
   pub ipfs_content: String,
   pub user_balance: String,  // Wei as string
}

#[derive(Debug, Serialize, Deserialize,ToSchema)]
pub struct OwnedToken {
   pub id: String,
   pub name: String,
   pub symbol: String,
   pub ipfs_content: String,
   pub user_balance: String,  // Wei as string
}

#[derive(Debug, Serialize, Deserialize,ToSchema)]
pub struct WatchlistToken {
   pub id: String,
   pub name: String,
   pub symbol: String,
   pub ipfs_content: String,
   pub user_balance: String,  // Wei as string
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Community {
   pub id: String,
   pub name: String,
   pub img_url: String,
}
