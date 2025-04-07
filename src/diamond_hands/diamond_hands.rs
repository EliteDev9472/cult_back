use rand::Rng;
use ordered_float::OrderedFloat;
use sqlx::{Postgres, Transaction};
use anyhow::Result;
use alloy::{primitives::{Address, B256, U256}, rlp::Encodable};
use chrono::{DateTime, Utc};
use std::collections::BinaryHeap;
use alloy_merkle_tree::tree::MerkleTree;
use actix_web::{web, App, HttpResponse, HttpServer};
//use cult_backend::auth::middleware::ApiGuard;
//use cult_backend::config::Settings;
//mod utils;
// In diamond_hands.rs

use crate::utils::update_contract_merkle_roots::update_contract_merkle_roots;

pub const SAMPLE_SIZE: usize = 10;
// Define constants for the diamond hand list "community"
const DIAMOND_HAND_ID: &str = "0x000000000000000000000000000000000d1a305d";
const DIAMOND_HAND_NAME: &str = "diamondHandList";
const DIAMOND_HAND_IMG: &str = "https://placeholder.com/diamond_hands.png";
const DIAMOND_HAND_ADDRESS: &str = "0x000000000000000000000000000000000d1a305d";
const DIAMOND_HAND_CHAIN: &str = "monadTestnet";


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

    let current_root = sqlx::query!(
        r#"
        SELECT merkle_root FROM communities 
        WHERE id = $1
        "#,
        DIAMOND_HAND_ID
    )
    .fetch_optional(&mut **tx)
    .await?;
    
    // if let Some(record) = current_root {
    //     if let Some(root) = record.merkle_root {
    //         // First update the current root in the contract with holder_count 0
    //         // This effectively invalidates the current merkle root
    //         let merkle_root_hex = format!("0x{}", hex::encode(&root));
    //         update_contract_merkle_roots(
    //             vec![merkle_root_hex], 
    //             vec![0]  // Set holder_count to 0
    //         ).await?;
    //     }
    // }
    
    // Clear existing entries
    sqlx::query("TRUNCATE diamond_hand_list")
        .execute(&mut **tx)
        .await?;

    if !account_ids.is_empty() {
        // Generate Merkle data
        let (merkle_root, timestamp, proofs, merkle_root_hex) = {
            let mut leaves = Vec::new();
            
            // Validate addresses and create leaves
            for account_id in &account_ids {
                let account_bytes = hex::decode(account_id.trim_start_matches("0x"))
                .map_err(|e| anyhow::anyhow!("Invalid hex in address {}: {}", account_id, e))?;
            
                if account_bytes.len() != 20 {
                    return Err(anyhow::anyhow!("Address {} is not 20 bytes", account_id));
                }
        
                // Pad address into 32-byte Merkle leaf
                let mut address_bytes = [0u8; 32];
                address_bytes[12..].copy_from_slice(&account_bytes);
                leaves.push(B256::from(address_bytes));
            }

            // Build Merkle tree
            let mut merkle_tree = MerkleTree::new();
            for leaf in &leaves {
                merkle_tree.insert(*leaf);
            }
            merkle_tree.finish();

        // Generate proofs as a map of address -> proof array
        let mut proofs_map = std::collections::HashMap::new();
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = merkle_tree.create_proof(leaf)
                .ok_or_else(|| anyhow::anyhow!("Failed to create proof for {}", account_ids[i]))?;

            // Convert proof to string array format
            let proof_path: Vec<String> = proof.siblings
                .iter()
                .map(|node| format!("{:?}", node))
                .collect();
            
            // Use address as key and proof array as value
            proofs_map.insert(account_ids[i].clone(), proof_path);
        }

        // Convert map to JSONB
        let all_proofs = serde_json::to_value(&proofs_map)?;
        let root_bytes = merkle_tree.root.to_vec();
        let root_hex = format!("0x{}", hex::encode(&root_bytes));
            (
                root_bytes,
                Utc::now(),
                all_proofs,
                root_hex
            )
        };

        // Single bulk insert with all data
        sqlx::query!(
            r#"
            INSERT INTO diamond_hand_list 
                (account_id,last_updated_time)
            SELECT 
                unnest($1::text[]), 
                $2
            "#,
            &account_ids,
            timestamp
        )
        .execute(&mut **tx)
        .await?;

        // Upsert into communities table
        sqlx::query!(
            r#"
            INSERT INTO communities 
                (id, name, img_url, address, chain, merkle_root, last_updated_time, merkle_proofs, holder_count)
            VALUES 
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) 
            DO UPDATE SET 
                merkle_root = $6,
                last_updated_time = $7,
                merkle_proofs = $8,
                holder_count = $9
            "#,
            DIAMOND_HAND_ID,
            DIAMOND_HAND_NAME,
            DIAMOND_HAND_IMG,
            DIAMOND_HAND_ADDRESS,
            DIAMOND_HAND_CHAIN,
            merkle_root,
            timestamp,
            proofs,
            SAMPLE_SIZE as i32
        )
        .execute(&mut **tx)
        .await?;

            // Step 3: Update contract with new merkle root and holder count of 10
            // update_contract_merkle_roots(
            //     vec![merkle_root_hex], 
            //     vec![SAMPLE_SIZE as u32]
            // ).await?;
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