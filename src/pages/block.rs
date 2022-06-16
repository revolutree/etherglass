use crate::{parser, RequestData};
use rocket::serde::{Deserialize, Serialize};
use rocket_dyn_templates::{context, Template};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(crate = "rocket::serde")]
pub struct ComplexBlock {
    pub base_fee_per_gas: String,
    pub difficulty: String,
    pub extra_data: String,
    pub gas_limit: String,
    pub gas_used: String,
    pub hash: String,
    pub logs_bloom: String,
    pub miner: String,
    pub mix_hash: String,
    pub nonce: String,
    pub number: String,
    pub parent_hash: String,
    pub receipts_root: String,
    pub sha3uncles: String,
    pub size: String,
    pub state_root: String,
    pub timestamp: String,
    pub total_difficulty: String,
    pub transactions: Vec<String>,
    pub transactions_root: String,
    pub uncles: Vec<serde_json::Value>,
}

#[get("/block/<block_number>?<with_tx>")]
pub async fn block(
    block_number: &str,
    with_tx: Option<String>,
    redis_cache: &crate::State<crate::Cache>,
) -> Template {
    let b_n = block_number.parse::<i64>().unwrap();
    let block = &parser::parse_request(
        "eth",
        "block",
        RequestData {
            data: serde_json::json!({ "blockNumber": b_n }),
        },
    )
    .await;

    let mut result = block.data["block"].clone();
    let b_h = result["hash"].to_string();
    let mut transactions: Vec<SimpleTransaction> = Vec::new();
    let mut existing_cache = false;
    if redis_cache.enabled {
        existing_cache = crate::rcache::check_cache(
            redis_cache.redis_client.clone().unwrap(),
            &format!("blocktx_{}", b_h),
        )
        .unwrap();
    }
    if with_tx.is_some() || existing_cache {
        transactions =
            retrieve_transactions(result["transactions"].clone(), redis_cache, b_h).await;
    } else {
        // could cache the transactions here anyway async
    }

    result["transactions"] =
        serde_json::from_str(&serde_json::to_string(&transactions).unwrap()).unwrap();

    Template::render("block", context! { block: result })
}

#[get("/block_hash/<block_hash>?<with_tx>")]
pub async fn block_hash(
    block_hash: &str,
    with_tx: Option<String>,
    redis_cache: &crate::State<crate::Cache>,
) -> Template {
    let b_h = crate::clean(block_hash.to_string());
    let block = &parser::parse_request(
        "eth",
        "blockByHash",
        RequestData {
            data: serde_json::json!({ "blockHash": b_h }),
        },
    )
    .await;

    let mut result = block.data["block"].clone();

    let mut transactions: Vec<SimpleTransaction> = Vec::new();
    if with_tx.is_some()
        || crate::rcache::check_cache(
            redis_cache.redis_client.clone().unwrap(),
            &format!("blocktx_{}", b_h),
        )
        .unwrap()
    {
        transactions =
            retrieve_transactions(result["transactions"].clone(), redis_cache, b_h).await;
    } else {
        // could cache the transactions anyway here asyncronously
    }
    result["transactions"] =
        serde_json::from_str(&serde_json::to_string(&transactions).unwrap()).unwrap();

    Template::render("block", context! { block: result })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(crate = "rocket::serde")]
pub struct SimpleTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub block_hash: String,
}

pub async fn retrieve_transactions(
    txs: serde_json::Value,
    redis_cache: &crate::Cache,
    block_hash: String,
) -> Vec<SimpleTransaction> {
    println!("  Retrieving transactions for block {}", block_hash);
    let transactions = txs.as_array().unwrap();
    let mut final_output: Vec<SimpleTransaction> = vec![];

    if redis_cache.enabled {
        let cache_result = crate::rcache::get(
            redis_cache.redis_client.clone().unwrap(),
            &format!("blocktx_{}", block_hash),
        );
        if cache_result.is_ok() {
            let res = cache_result.unwrap();
            if res != "" {
                let cache_result = res;
                let cache_result = serde_json::from_str(&cache_result).unwrap();
                return cache_result;
            }
        }
    }

    for tx in transactions {
        let t_h = crate::clean(tx.to_string());
        let transaction = &parser::parse_request(
            "eth",
            "transaction",
            RequestData {
                data: serde_json::json!({ "tx": t_h }),
            },
        )
        .await;

        let result = transaction.data["transaction"].clone();
        final_output.push(SimpleTransaction {
            hash: crate::clean(result["hash"].to_string()),
            from: crate::clean(result["from"].to_string()),
            to: crate::clean(result["to"].to_string()),
            value: crate::clean(result["value"].to_string()),
            block_hash: crate::clean(result["blockHash"].to_string()),
        });
    }

    if redis_cache.enabled {
        let _ = crate::rcache::set(
            redis_cache.redis_client.clone().unwrap(),
            &format!("blocktx_{}", block_hash),
            &serde_json::to_string(&final_output).unwrap(),
        );
    }

    final_output
}
