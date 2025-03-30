use rand::Rng;
use ordered_float::OrderedFloat;
use sqlx::{Postgres, Transaction};
use anyhow::Result;
use std::collections::BinaryHeap;

use actix_web::{web, App, HttpResponse, HttpServer};
//use cult_backend::auth::middleware::ApiGuard;
//use cult_backend::config::Settings;

pub const SAMPLE_SIZE: usize = 10;

pub async fn get_eligible_accounts(
    tx: &mut Transaction<'_, Postgres>
) -> Result<Vec<(String, i32)>> {
    let accounts = sqlx::query!(
        r#"
        SELECT id, diamond_hand_probability 
        FROM account 
        WHERE diamond_hand_probability > 0
        "#
    )
    .fetch_all(&mut **tx)
    .await?;

    Ok(accounts
        .into_iter()
        .map(|row| (row.id, row.diamond_hand_probability))
        .collect())
}

pub fn select_weighted_sample(accounts: Vec<(String, i32)>, sample_size: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut heap = BinaryHeap::with_capacity(sample_size);

    for (id, prob) in accounts {
        let weight = prob as f64;
        let key = -rng.gen::<f64>().ln() / (weight/100.0);
        let item = (OrderedFloat(key), id);

        if heap.len() < sample_size {
            heap.push(item);
        } else if let Some(top) = heap.peek() {
            if item.0 < top.0 {
                heap.pop();
                heap.push(item);
            }
        }
    }

    heap.into_iter().map(|(_key, id)| id).collect()
}

pub async fn update_diamond_hands(
    tx: &mut Transaction<'_, Postgres>,
    account_ids: Vec<String>
) -> Result<()> {
    println!("Updating Diamond Hands");
    sqlx::query("TRUNCATE diamond_hand_list")
        .execute(&mut **tx)
        .await?;
    
    if !account_ids.is_empty() {
        sqlx::query(
            "INSERT INTO diamond_hand_list (account_id) SELECT unnest($1::text[])"
        )
        .bind(&account_ids)
        .execute(&mut **tx)
        .await?;
    }
    
    Ok(())
}

fn main(){}

// #[tokio::main]
// async fn main() -> Result<()> {
//     dotenv::dotenv().ok();

//     let settings = Settings::new().expect("Failed to load settings");

    
//     HttpServer::new(move || {
//         App::new()
//             .wrap(ApiGuard::new())
//             .service(
//                 web::resource("/run-diamond-hands")
//                     .route(web::post().to(|| async {
//                         let result: Result<_, Box<dyn std::error::Error>> = async {
//                             let database_url = std::env::var("DATABASE_URL")?;
//                             let pool = sqlx::PgPool::connect(&database_url).await?;

//                             let mut tx = pool.begin().await?;
//                             let accounts = get_eligible_accounts(&mut tx).await?;
//                             let selected = select_weighted_sample(accounts, SAMPLE_SIZE);
                            
//                             update_diamond_hands(&mut tx, selected).await?;
//                             tx.commit().await?;
//                             Ok(())
//                         }.await;

//                         match result {
//                             Ok(_) => HttpResponse::Ok().body("Diamond hands executed"),
//                             Err(e) => HttpResponse::InternalServerError().body(e.to_string())
//                         }
//                     }))
//             )
//     })
//     .bind(&settings.bind_address)?
//     .run()
//     .await
// }