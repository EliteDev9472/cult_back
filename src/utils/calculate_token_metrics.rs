use sqlx::{postgres::PgPool, Error};
use std::env;

async fn update_balance_metrics(pool: &PgPool) -> Result<(), Error> {
    // Calculate PNL and current value for each position
    sqlx::query!(
        r#"
        UPDATE token_balance tb
        SET 
            pnl = COALESCE(
                (SELECT SUM(
                    CASE 
                        WHEN tt.trade_type = 'Sell' THEN tt.eth_amount - tt.eth_fee
                        ELSE - (tt.eth_amount + tt.eth_fee)
                    END
                ) FROM token_trade tt
                WHERE tt.trader_id = tb.account_id AND tt.token_id = tb.token_id),
                0
            ),
            current_value = tb.value * (
                SELECT eth_amount/token_amount 
                FROM token_trade 
                WHERE token_id = tb.token_id 
                ORDER BY timestamp DESC 
                LIMIT 1
            )
        "#
    ).execute(pool).await?;

    Ok(())
}

async fn calculate_token_stats(pool: &PgPool) -> Result<(), Error> {
    // Calculate per-token statistics
    sqlx::query!(
        r#"
        INSERT INTO token_stats (
            token_id, 
            mean_duration, 
            stddev_duration, 
            mean_pnl, 
            stddev_pnl,
            mean_value,
            stddev_value
        )
        SELECT
            token_id,
            AVG(held_for),
            STDDEV(held_for),
            AVG(pnl),
            STDDEV(pnl),
            AVG(current_value),
            STDDEV(current_value)
        FROM token_balance
        WHERE pnl IS NOT NULL
        GROUP BY token_id
        ON CONFLICT (token_id) DO UPDATE SET
            mean_duration = EXCLUDED.mean_duration,
            stddev_duration = EXCLUDED.stddev_duration,
            mean_pnl = EXCLUDED.mean_pnl,
            stddev_pnl = EXCLUDED.stddev_pnl,
            mean_value = EXCLUDED.mean_value,
            stddev_value = EXCLUDED.stddev_value
        "#
    ).execute(pool).await?;

    // Update liquidity and holder weights
    sqlx::query!(
        r#"
        WITH liquidity AS (
            SELECT 
                token_id,
                LOG(SUM(eth_amount)) AS liquidity_score
            FROM token_trade
            GROUP BY token_id
        ),
        holders AS (
            SELECT
                token_id,
                LOG(holder_count) AS holder_weight
            FROM cult_token
        )
        UPDATE token_stats ts
        SET
            liquidity_score = l.liquidity_score,
            holder_weight = h.holder_weight
        FROM liquidity l
        JOIN holders h ON ts.token_id = h.token_id
        WHERE ts.token_id = l.token_id
        "#
    ).execute(pool).await?;

    Ok(())
}

async fn calculate_z_scores(pool: &PgPool) -> Result<(), Error> {
    // Calculate normalized z-scores directly in token_balance
    sqlx::query!(
        r#"
        UPDATE token_balance tb
        SET
            duration_z = (tb.held_for - ts.mean_duration) / NULLIF(ts.stddev_duration, 0),
            pnl_z = (tb.pnl - ts.mean_pnl) / NULLIF(ts.stddev_pnl, 0),
            value_z = (tb.current_value - ts.mean_value) / NULLIF(ts.stddev_value, 0)
        FROM token_stats ts
        WHERE tb.token_id = ts.token_id
        AND ts.stddev_duration > 0
        AND ts.stddev_pnl > 0
        "#
    ).execute(pool).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let pool = PgPool::connect(&env::var("DATABASE_URL").unwrap()).await?;
    
    update_balance_metrics(&pool).await?;
    calculate_token_stats(&pool).await?;
    calculate_z_scores(&pool).await?;
    
    println!("Reputation metrics calculated successfully");
    Ok(())
}