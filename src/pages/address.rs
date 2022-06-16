use crate::{parser, RequestData};
use rocket::serde::{Deserialize, Serialize};
use rocket_dyn_templates::{context, Template};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(crate = "rocket::serde")]
pub struct SimpleAddress {
    pub address: String,
    pub balance: String,
    pub transactions: Vec<SimpleTransaction>,
}

#[get("/address/<address_hex>")]
pub async fn address(address_hex: &str) -> Template {
    let a = crate::clean(address_hex.to_string());
    let address = &parser::parse_request(
        "eth",
        "balance",
        RequestData {
            data: serde_json::json!({ "address": a }),
        },
    )
    .await;

    let result = address.data.clone();
    Template::render("address", context! { address: result })
}

use crate::pages::block::retrieve_transactions;

use super::block::SimpleTransaction;

pub async fn get_balance(address: &str) -> String {
    let address = &parser::parse_request(
        "eth",
        "balance",
        RequestData {
            data: serde_json::json!({ "address": address }),
        },
    )
    .await;

    let balance = address.data.clone()["balance"].to_string();
    return balance;
}

/// This function is used to retrieve the transactions of an address.
/// It is used in the block page and the address page.
/// It is also used in the crawler to retrieve the transactions of a block.
/// note: could be much better implemented.
pub async fn cache_addresses_transactions_from_block(block_number: i64) {
    // This whole cache should be coming from the Rocket handler, not here.
    let redis_cache = crate::Cache {
        enabled: *crate::REDIS_CACHE.lock().unwrap(),

        // Temporary redefining it here, should be moved around coming from the Rocket handler
        redis_client: Some(redis::Client::open("redis://localhost:6379").unwrap()),
    };

    // if indexed block is already cached, don't do anything
    if redis_cache.enabled {
        if crate::rcache::check_cache(
            redis_cache.redis_client.clone().unwrap(),
            &block_number.to_string(),
        )
        .unwrap()
        {
            println!("BLOCK {} ALREADY CACHED", block_number);
            return;
        }
    }

    let block = &parser::parse_request(
        "eth",
        "block",
        RequestData {
            data: serde_json::json!({ "blockNumber": block_number }),
        },
    )
    .await;
    let result = block.data["block"].clone();

    let b_h = result["hash"].to_string();
    let transactions =
        retrieve_transactions(result["transactions"].clone(), &redis_cache, b_h.clone()).await;

    for t in transactions {
        let t_h = t.hash;
        let t_f = t.from;
        let t_t = t.to;
        let t_a = t.value;

        // if transaction is already indexed
        if crate::rcache::check_cache(
            redis_cache.redis_client.clone().unwrap(),
            &format!("indexed_{}", t_h),
        )
        .unwrap()
        {
            println!("TX {} ALREADY CACHED", t_h);
            continue;
        }
        // if address exists in redis

        if crate::rcache::check_cache(redis_cache.redis_client.clone().unwrap(), &t_f).unwrap() {
            // deserialize string into SimpleAddress
            let cached_address = crate::rcache::get(
                redis_cache.redis_client.clone().unwrap(),
                &format!("address_{}", t_f),
            )
            .unwrap();
            let mut cached_address: SimpleAddress = serde_json::from_str(&cached_address).unwrap();

            cached_address.transactions.push(SimpleTransaction {
                block_hash: b_h.clone(),
                hash: t_h.clone(),
                from: t_f.clone(),
                to: t_t.clone(),
                value: t_a.clone(),
            });

            // serialize SimpleAddress back to string
            let cached_address = serde_json::to_string(&cached_address).unwrap();

            println!("SAVING TX OUTBOUND {} to ADDRESS {}", t_h, t_f);

            // write it back on redis
            let _ = crate::rcache::set(
                redis_cache.redis_client.clone().unwrap(),
                &format!("address_{}", t_f.clone()),
                &cached_address,
            );
        } else {
            // create new SimpleAddress
            let new_address = SimpleAddress {
                address: t_f.clone(),
                balance: "0".to_string(), // no cache for balance
                transactions: vec![SimpleTransaction {
                    block_hash: b_h.clone(),
                    hash: t_h.clone(),
                    from: t_f.clone(),
                    to: t_t.clone(),
                    value: t_a.clone(),
                }],
            };

            // serialize SimpleAddress back to string
            let new_address = serde_json::to_string(&new_address).unwrap();

            // write it back on redis
            let _ = crate::rcache::set(
                redis_cache.redis_client.clone().unwrap(),
                &format!("address_{}", t_f.clone()),
                &new_address,
            );
        }

        // if address exists in redis
        if crate::rcache::check_cache(redis_cache.redis_client.clone().unwrap(), &t_t).unwrap() {
            // deserialize string into SimpleAddress
            let cached_address = crate::rcache::get(
                redis_cache.redis_client.clone().unwrap(),
                &format!("address_{}", t_t),
            )
            .unwrap();
            let mut cached_address: SimpleAddress = serde_json::from_str(&cached_address).unwrap();

            cached_address.transactions.push(SimpleTransaction {
                block_hash: b_h.clone(),
                hash: t_h.clone(),
                from: t_f.clone(),
                to: t_t.clone(),
                value: t_a.clone(),
            });

            // serialize SimpleAddress back to string
            let cached_address = serde_json::to_string(&cached_address).unwrap();

            println!("SAVING TX INBOUD {} to ADDRESS {}", t_h, t_t);

            // write it back on redis
            let _ = crate::rcache::set(
                redis_cache.redis_client.clone().unwrap(),
                &format!("address_{}", t_t.clone()),
                &cached_address,
            );
        } else {
            // create new SimpleAddress
            let new_address = SimpleAddress {
                address: t_t.clone(),
                balance: "0".to_string(), // no cache for balance
                transactions: vec![SimpleTransaction {
                    block_hash: b_h.clone(),
                    hash: t_h.clone(),
                    from: t_f.clone(),
                    to: t_t.clone(),
                    value: t_a.clone(),
                }],
            };

            // serialize SimpleAddress back to string
            let new_address = serde_json::to_string(&new_address).unwrap();

            // write it back on redis
            let _ = crate::rcache::set(
                redis_cache.redis_client.clone().unwrap(),
                &format!("address_{}", t_t.clone()),
                &new_address,
            );
        }
        let _ = crate::rcache::set(
            redis_cache.redis_client.clone().unwrap(),
            &format!("indexedtx_{}", t_h.clone()),
            &"1".to_string(),
        );
    }
    let _ = crate::rcache::set(
        redis_cache.redis_client.clone().unwrap(),
        &format!("indexedblock_{}", b_h.clone()),
        &"1".to_string(),
    );
}
