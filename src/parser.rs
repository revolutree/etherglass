use rocket::serde::Serialize;
use std::str::FromStr;

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct ResponseData {
    pub data: serde_json::Value,
}

pub async fn parse_request(api: &str, method: &str, data: crate::RequestData) -> ResponseData {
    let client = crate::client().await.unwrap();

    // This whole cache should be coming from the Rocket handler, not here.
    let redis_cache = crate::Cache {
        enabled: *crate::REDIS_CACHE.lock().unwrap(),

        // Temporary redefining it here, should be moved around coming from the Rocket handler
        redis_client: Some(redis::Client::open("redis://localhost:6379").unwrap()),
    };

    let result = match api {
        "eth" => match method {
            "balance" => {
                let input_address = data.data["address"].as_str().unwrap();
                let mut _balance = web3::types::U256::from(0 as i128);
                let mut _address = web3::types::H160::from([0u8; 20]);
                if input_address.contains(".eth") {
                    _address = client.ens.eth_address(input_address).await.unwrap();
                    _balance = client.web3.eth().balance(_address, None).await.unwrap();
                } else {
                    _address = input_address.parse().unwrap();
                    _balance = client.web3.eth().balance(_address, None).await.unwrap();
                }
                ResponseData {
                    data: serde_json::json!({
                        "balance": _balance,
                        "address":_address.to_string()
                    }),
                }
            }
            "blockNumber" => {
                let block_number = client.web3.eth().block_number().await.unwrap();
                ResponseData {
                    data: serde_json::json!({ "blockNumber": block_number }),
                }
            }
            "block" => {
                let block_number = &data.data["blockNumber"]
                    .clone()
                    .to_string()
                    .parse::<u64>()
                    .unwrap();
                let web3_block_number = web3::types::BlockNumber::from(*block_number as i64);

                if redis_cache.enabled {
                    if crate::rcache::check_cache(
                        redis_cache.redis_client.clone().unwrap(),
                        &block_number.to_string(),
                    )
                    .unwrap()
                    {
                        let cached_block = crate::rcache::get(
                            redis_cache.redis_client.clone().unwrap(),
                            &format!("block_{}", block_number),
                        )
                        .unwrap();
                        return ResponseData {
                            data: serde_json::from_str(&cached_block).unwrap(),
                        };
                    };
                }

                let block = client
                    .web3
                    .eth()
                    .block(web3::types::BlockId::from(web3_block_number))
                    .await
                    .unwrap();

                if redis_cache.enabled {
                    let _ = crate::rcache::set(
                        redis_cache.redis_client.clone().unwrap(),
                        &format!("block_{}", block_number),
                        &serde_json::to_string(&block).unwrap(),
                    );
                }
                ResponseData {
                    data: serde_json::json!({ "block": block }),
                }
            }
            "blockByHash" => {
                let block_hash = &crate::clean(data.data["blockHash"].clone().to_string())
                    .parse::<web3::types::H256>()
                    .unwrap();

                if redis_cache.enabled {
                    if crate::rcache::check_cache(
                        redis_cache.redis_client.clone().unwrap(),
                        &block_hash.to_string(),
                    )
                    .unwrap()
                    {
                        let cached_block = crate::rcache::get(
                            redis_cache.redis_client.clone().unwrap(),
                            &format!("block_{}", block_hash.to_string()),
                        )
                        .unwrap();
                        return ResponseData {
                            data: serde_json::from_str(&cached_block).unwrap(),
                        };
                    };
                }

                let block = client
                    .web3
                    .eth()
                    .block(web3::types::BlockId::Hash(*block_hash))
                    .await
                    .unwrap();

                if redis_cache.enabled {
                    let _ = crate::rcache::set(
                        redis_cache.redis_client.clone().unwrap(),
                        &format!("block_{}", block_hash.to_string()),
                        &serde_json::to_string(&block).unwrap(),
                    );
                }

                ResponseData {
                    data: serde_json::json!({ "block": block }),
                }
            }
            "transaction" => {
                let tx_hash =
                    web3::types::H256::from_str(data.data["tx"].as_str().unwrap()).unwrap();
                let transaction = client
                    .web3
                    .eth()
                    .transaction(web3::types::TransactionId::Hash(tx_hash))
                    .await
                    .unwrap();
                ResponseData {
                    data: serde_json::json!({ "transaction": transaction }),
                }
            }
            "syncing" => {
                let syncing = client.web3.eth().syncing().await.unwrap();
                ResponseData {
                    data: serde_json::json!({ "syncing": syncing }),
                }
            }
            "chainId" => {
                let chain_id = client.web3.eth().chain_id().await.unwrap();
                ResponseData {
                    data: serde_json::json!({ "chainId": chain_id }),
                }
            }
            _ => ResponseData {
                data: serde_json::json!({}),
            },
        },
        _ => ResponseData {
            data: serde_json::json!({}),
        },
    };

    response_to_human_readable(result)
}

pub fn response_to_human_readable(res: ResponseData) -> ResponseData {
    let mut r = res.clone();

    let mut data = r.data;
    let data_iter = data.clone();

    for (key, _value) in data_iter.as_object().unwrap() {
        let strkey = key.as_str();
        match strkey {
            "block" => {
                for (k, v) in data_iter["block"].as_object().unwrap() {
                    let str_k = k.as_str();
                    match str_k {
                        "gasUsed" => {
                            data["block"]["gasUsed"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        "gasLimit" => {
                            data["block"]["gasLimit"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        "number" => {
                            data["block"]["number"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        _ => {}
                    }
                }
            }
            "transaction" => {
                for (k, v) in data_iter["transaction"].as_object().unwrap() {
                    let str_k = k.as_str();
                    match str_k {
                        "value" => {
                            data["transaction"]["value"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        "blockNumber" => {
                            data["transaction"]["blockNumber"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        "gas" => {
                            data["transaction"]["gas"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        "gasPrice" => {
                            data["transaction"]["gasPrice"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        "transactionIndex" => {
                            data["transaction"]["transactionIndex"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        "nonce" => {
                            data["transaction"]["nonce"] = serde_json::Value::from(
                                crate::json_value_hex_to_int(v.clone()).to_string(),
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    r.data = data;

    return r;
}
