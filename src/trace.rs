// All the packages I'll be using in this file
use anvil::eth::fees::calculate_next_block_base_fee;
use anyhow::{anyhow, Result};
use cfmms::{
    checkpoint::sync_pools_from_checkpoint,
    dex::{Dex, DexVariant},
    pool::Pool,
    sync::sync_pairs,
};
use dashmap::DashMap;
use ethers::{
    abi,
    providers::{Provider, Ws},
    types::{Address, BlockNumber, Diff, TraceType, Transaction, H160, H256, U256, U64},
    utils::keccak256,
};
use ethers_providers::Middleware;
use log::info;
use std::{path::Path, str::FromStr, sync::Arc};
use tokio::sync::broadcast::{self, Sender};
use tokio::task::JoinSet;
use tokio_stream::StreamExt;

// Create this function first
pub async fn mempool_watching(target_address: String) -> Result<()> {
    // Setup: Create the WS provider and wrap it in Arc
    let wss_url: String = std::env::var("WSS_URL").unwrap();
    let provider = Provider::<Ws>::connect(wss_url).await?;
    let provider = Arc::new(provider);

    // Step #1: Using cfmms-rs to sync all pools created on Uniswap V3
    let checkpoint_path = ".cfmms-checkpoint.json";
    let checkpoint_exists = Path::new(checkpoint_path).exists();

    let pools = DashMap::new();

    let dexes_data = [(
        // Uniswap v3
        "0x1F98431c8aD98523631AE4a59f267346ea31F984",
        DexVariant::UniswapV3,
        12369621u64,
    )];
    let dexes: Vec<_> = dexes_data
        .into_iter()
        .map(|(address, variant, number)| {
            Dex::new(H160::from_str(address).unwrap(), variant, number, Some(300))
        })
        .collect();

    let pools_vec = if checkpoint_exists {
        let (_, pools_vec) =
            sync_pools_from_checkpoint(checkpoint_path, 100000, provider.clone()).await?;
        pools_vec
    } else {
        sync_pairs(dexes.clone(), provider.clone(), Some(checkpoint_path)).await?
    };

    for pool in pools_vec {
        pools.insert(pool.address(), pool);
    }

    info!("Uniswap V3 pools synced: {}", pools.len());
    Ok(())
}