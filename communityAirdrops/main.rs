use alloy::primitives::{Address, B256, U256};
use alloy_merkle_tree::tree::MerkleTree;
use chrono::{DateTime, Utc};
use dotenv::dotenv;
//use reqwest::Error;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::fs::File;
use std::io::BufReader;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use serde_json::from_reader;
use sqlx::PgPool;
mod handler;
use sqlx::FromRow;
use std::time::{Instant}; //to measure time for certain operations


#[derive(Debug, FromRow)] 
pub struct Community {
    pub id: String, 
    pub name: String,    // Unique name for the community
    pub img_url:String,// Unique image for the community
    pub address: String, // Address of the community (unique identifier)
    pub chain: String,   // Blockchain chain (e.g., "Ethereum", "Polygon")
    pub merkle_root: Option<Vec<u8>>, // Merkle root (calculated later)
    pub last_updated_time: Option<chrono::DateTime<Utc>>, // Last updated timestamp
    pub merkle_proofs: Option<serde_json::Value>, // Address -> Merkle proofs
}
#[derive(Debug, Deserialize)]
pub struct NftOwnersResponse {
    pub owners: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CommunityConfig {
    name: String,
    address: String,
    chain: String,
    img_url:String
}


impl Community {
    pub fn new(name: String, img_url:String, address: String, chain: String) -> Result<Self, String> {
        // Validate the address using Alloy
        if Address::from_str(&address).is_err() {
            return Err("Invalid Ethereum address".to_string());
        }

        Ok(Self {
            id:address.clone(),
            name,
            img_url,
            address,
            chain,
            merkle_root: None,
            last_updated_time: None,
            merkle_proofs: None,
        })
    }

    // Method to calculate and set the merkle root and proofs
    pub fn set_merkle_data(&mut self, leaves: Vec<String>) -> Result<(), String> {
        // Convert leaves to B256
        let b256_leaves: Vec<B256> = leaves
            .iter()
            .enumerate()
            .map(|(i, _)| B256::from(U256::from(i))) // Convert index to B256
            .collect();

        // Create a Merkle tree
        let mut tree = MerkleTree::new();

        // Insert leaves into the tree
        for leaf in &b256_leaves {
            tree.insert(*leaf);
        }

        // Finalize the tree
        tree.finish();

        // Calculate the Merkle root
        let merkle_root = tree.root.to_vec(); 
        self.merkle_root = Some(merkle_root);

        // Calculate Merkle proofs for each address
        let mut proofs = HashMap::new();
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree
                .create_proof(&b256_leaves[i])
                .ok_or_else(|| format!("Failed to create proof for leaf at index {}", i))?;

            // Convert proof to Vec<String>
            let proof_strings: Vec<String> = proof
                .siblings
                .iter()
                .map(|p| format!("{:?}", p))
                .collect();

            proofs.insert(leaf.clone(), proof_strings);
        }

        self.merkle_proofs = serde_json::to_value(&proofs).map(Some).map_err(|e| e.to_string())?;

        self.last_updated_time = Some(Utc::now());

        Ok(())
    }
}

pub async fn fetch_nft_holders(
    config: &CommunityConfig, 
    api_key: &str
) -> Result<Vec<String>, anyhow::Error> {
    let url = format!(
        "https://{}.g.alchemy.com/nft/v3/{}/getOwnersForContract?contractAddress={}&withTokenBalances=false",
        config.chain, api_key, config.address
    );

    let response = reqwest::get(&url).await?;

    let response: NftOwnersResponse = response.json().await?;
    
    let owners = response.owners;
    Ok(owners)
}

pub async fn update_all_communities(pool: &PgPool, api_key: &str) -> Result<(), anyhow::Error> {
    let mut path = PathBuf::from(std::env::current_dir()?);
    path.push("communityAirdrops/communities.json");
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let configs: Vec<CommunityConfig> = from_reader(reader)?;

    for config in configs {
        let mut tx = pool.begin().await?;

        // Fetch existing community with its ID
        let existing_community = handler::get_community_by_address(&mut tx, &config.address).await?;
        let start_time = Instant::now();
        // Fetch current NFT holders from blockchain
        let owners = fetch_nft_holders(&config, api_key).await?;
        
        // Create temporary community to calculate Merkle data
        let mut temp_community = Community::new(
            config.name.clone(),
            config.img_url.clone(),
            config.address.clone(),
            config.chain.clone()
        ).map_err(|e| anyhow!("Community creation error: {}", e))?;
        
        temp_community.set_merkle_data(owners.clone())
            .map_err(|e| anyhow!("Merkle data calculation failed: {}", e))?;
        let duration = start_time.elapsed();
        println!("Processing took: {:?}", duration); 
        // Update or create community in database
        let db_community = match existing_community {
            Some(existing) => {
                handler::update_community(
                    &mut tx,
                    &existing.address,
                    temp_community.merkle_root.as_ref(),
                    temp_community.merkle_proofs.as_ref(),
                    temp_community.last_updated_time
                ).await?
            }
            None => {
                handler::create_community(
                    &mut tx,
                    &temp_community.name,
                    &temp_community.img_url,
                    &temp_community.address,
                    &temp_community.chain,
                    temp_community.merkle_root.as_ref(),
                    temp_community.merkle_proofs.as_ref(),
                    temp_community.last_updated_time
                ).await?
            }
        };
        println!("Created community");
        
        handler::generate_dummy_account_data(
            &mut tx,
            &owners
        ).await?;

        // Atomic membership refresh
        handler::refresh_community_memberships(
            &mut tx,
            db_community.id,
            &owners
        ).await?;

        tx.commit().await?;
    }
    
    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load environment variables from .env file
    dotenv().ok();
    let api_key = env::var("ALCHEMY_API_KEY").expect("ALCHEMY_API_KEY must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"); 
    let pool = PgPool::connect(&database_url).await?;
    
    // can run cron job on top of this to update communities every day
    //TODO: also add a function to update account data
    update_all_communities(&pool, &api_key).await?;

    Ok(())
}