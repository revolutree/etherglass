pub async fn crawler(s_b: i64, e_b: i64) {
    println!("Starting tx crawler");

    let block_number = &crate::parser::parse_request(
        "eth",
        "blockNumber",
        crate::RequestData {
            data: serde_json::json!({}),
        },
    )
    .await
    .data["blockNumber"];
    let b_n = crate::json_value_hex_to_int(block_number.clone());
    let latest_block: i64 = b_n as i64;
    let mut starting_block = s_b;
    if starting_block == 0 {
        starting_block = latest_block - 100;
    }

    for i in starting_block..=latest_block {
        println!("Crawling tx from block {}", i);
        if i >= e_b {
            break;
        }
        let _ = crate::pages::address::cache_addresses_transactions_from_block(i).await;
    }

    println!("Finished tx crawler");
}
