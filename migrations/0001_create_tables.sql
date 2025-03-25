-- CultToken Table
CREATE TABLE cult_token (
    id TEXT PRIMARY KEY, -- Changed to TEXT for address
    factory_address TEXT NOT NULL,
    token_creator TEXT NOT NULL,
    protocol_fee_recipient TEXT NOT NULL,
    bonding_curve TEXT NOT NULL,
    token_uri TEXT NOT NULL,
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    token_address TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    transaction_hash TEXT NOT NULL,
    holder_count BIGINT NOT NULL,
    airdrop_contract TEXT NOT NULL
);

-- TokenIPFSData Table
CREATE TABLE token_ipfs_data (
    id TEXT PRIMARY KEY, -- Changed to TEXT for concatenated hash and address
    hash TEXT NOT NULL,
    content TEXT NOT NULL,
    token_id TEXT REFERENCES cult_token(id) ON DELETE CASCADE -- Changed to TEXT
);

-- Account Table
CREATE TABLE account (
    id TEXT PRIMARY KEY, -- Changed to TEXT for address
    slug TEXT,
    referral_code TEXT,
    diamond_hand_probability INT NOT NULL,
    referrer_id TEXT REFERENCES account(id) ON DELETE SET NULL, -- Changed to TEXT
    total_referrals INT,
    fee_collected BIGINT NOT NULL,
    twitter TEXT,
    discord TEXT
);

CREATE TABLE diamond_hand_list (
    account_id TEXT PRIMARY KEY REFERENCES account(id),
    selected_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- TokenBalance Table
CREATE TABLE token_balance (
    account_id TEXT REFERENCES account(id),
    token_id TEXT REFERENCES cult_token(id),
    holding_duration BIGINT NOT NULL,
    pnl NUMERIC NOT NULL,
    holdings_value NUMERIC NOT NULL,
    duration_z NUMERIC,
    pnl_z NUMERIC,
    value_z NUMERIC,
    PRIMARY KEY (account_id, token_id)
);

--TODO: New Table
CREATE TABLE token_stats (
    token_id TEXT PRIMARY KEY REFERENCES cult_token(id),
    mean_duration NUMERIC,
    stddev_duration NUMERIC,
    mean_pnl NUMERIC,
    stddev_pnl NUMERIC,
    mean_value NUMERIC,
    stddev_value NUMERIC,
    liquidity_score NUMERIC,
    holder_weight NUMERIC
);

-- TradeType Enum
CREATE TYPE trade_type AS ENUM ('Buy', 'Sell');

-- TokenTrade Table
CREATE TABLE token_trade (
    id TEXT PRIMARY KEY, -- Changed to TEXT for concatenated hash and address
    token_id TEXT REFERENCES cult_token(id) ON DELETE CASCADE, -- Changed to TEXT
    trade_type trade_type NOT NULL,
    trader_id TEXT REFERENCES account(id) ON DELETE CASCADE, -- Changed to TEXT
    recipient_id TEXT REFERENCES account(id) ON DELETE CASCADE, -- Changed to TEXT
    order_referrer_id TEXT REFERENCES account(id) ON DELETE CASCADE, -- Changed to TEXT
    total_eth BIGINT NOT NULL,
    eth_fee BIGINT NOT NULL,
    eth_amount BIGINT NOT NULL,
    token_amount BIGINT NOT NULL,
    trader_token_balance BIGINT NOT NULL,
    total_supply BIGINT NOT NULL,
    market_type BIGINT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    transaction_hash TEXT NOT NULL
);

-- NOT USING RN TO SEE IF COMPUTING ON THE FLY MIGHT BE BETTER
-- New table to store OHLCV values (one example approach)
CREATE TABLE token_ohlcv (
    token_id TEXT REFERENCES cult_token(id) ON DELETE CASCADE,
    interval_start TIMESTAMPTZ NOT NULL,   -- e.g., daily/hourly bucket start time
    open NUMERIC,
    high NUMERIC,
    low NUMERIC,
    close NUMERIC,
    volume NUMERIC,
    PRIMARY KEY (token_id, interval_start)
);

-- Communities Table (unchanged)
CREATE TABLE communities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    img_url TEXT NOT NULL,
    address TEXT NOT NULL UNIQUE,
    chain TEXT NOT NULL,
    merkle_root BYTEA, 
    last_updated_time TIMESTAMPTZ, 
    merkle_proofs JSONB 
);

-- Account_Communities Table 
CREATE TABLE account_communities (
    account_id TEXT REFERENCES account(id),
    community_id TEXT REFERENCES communities(id),
    PRIMARY KEY (account_id, community_id)
);

-- Account_watchlists
CREATE TABLE account_watchlist (
    account_id TEXT NOT NULL REFERENCES account(id) ON DELETE CASCADE,
    cult_token_id TEXT NOT NULL REFERENCES cult_token(id) ON DELETE CASCADE,
    PRIMARY KEY (account_id, cult_token_id)
);

-- Indexes 
CREATE INDEX idx_account_communities_account ON account_communities(account_id);
CREATE INDEX idx_account_communities_community ON account_communities(community_id);

-- Index for retrieving trades by token more efficiently
CREATE INDEX idx_token_trade_token_id ON token_trade (token_id);


-- Materialized View for account_communities
CREATE MATERIALIZED VIEW account_community_summary AS
SELECT a.id, array_agg(c.address) AS communities
FROM account a
JOIN account_communities ac ON a.id = ac.account_id
JOIN communities c ON ac.community_id = c.id
GROUP BY a.id;

-- Materialized View for global_rankings
CREATE MATERIALIZED VIEW global_rankings AS
SELECT 
    a.id AS user_id,
    SUM(tb.duration_z * ts.liquidity_score) AS duration_score,
    SUM(tb.pnl_z * ts.holder_weight) AS pnl_score,
    SUM(tb.value_z * ts.liquidity_score) AS holdings_score,
    RANK() OVER (ORDER BY SUM(tb.duration_z * 0.4 + tb.pnl_z * 0.5 + tb.value_z * 0.1) DESC) AS global_rank
FROM token_balance tb
JOIN token_stats ts ON tb.token_id = ts.token_id
JOIN account a ON tb.account_id = a.id
GROUP BY a.id;