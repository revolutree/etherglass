use crate::{json_value_hex_to_int, parser, RequestData};
use rocket::serde::{Serialize, Deserialize};
use rocket_dyn_templates::{context, Template};

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, UriDisplayQuery))]
#[serde(crate = "rocket::serde")]
pub struct SimpleBlock {
    hash: String,
    number: i64,
    tx_amount: i64,
    human_date: i64,
}

const LATEST_BLOCKS_AMOUNT: i128 = 20;

#[get("/")]
pub async fn index() -> Template {
    let block_number = &parser::parse_request(
        "eth",
        "blockNumber",
        RequestData {
            data: serde_json::json!({}),
        },
    )
    .await
    .data["blockNumber"];
    let b_n = json_value_hex_to_int(block_number.clone());

    
    let latest_blocks: Vec<SimpleBlock> = get_latest_blocks(b_n).await;
    
    Template::render("index", context! { blocks: latest_blocks })
}

pub async fn get_latest_blocks(b_n: i128) -> Vec<SimpleBlock> {
    let mut latest_blocks: Vec<SimpleBlock> = vec![];
    for i in 0..LATEST_BLOCKS_AMOUNT {
        let block = &parser::parse_request(
            "eth",
            "block",
            RequestData {
                data: serde_json::json!({ "blockNumber": (b_n - i) as i64 }),
            },
        )
        .await;
        let block_hash = block.data["block"]["hash"].to_string();
        let tx_amount = block.data["block"]["transactions"]
            .as_array()
            .unwrap()
            .len() as i64;
        let block_timestamp =
            crate::json_value_hex_to_int(block.data["block"]["timestamp"].clone());
        latest_blocks.push(SimpleBlock {
            hash: crate::clean(block_hash.as_str().to_string()),
            number: (b_n - i) as i64,
            tx_amount: tx_amount,
            human_date: block_timestamp as i64,
        });
    }
    return latest_blocks
}