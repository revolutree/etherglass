use crate::{json_value_hex_to_int, parser, RequestData};
use rocket::serde::Serialize;
use rocket_dyn_templates::{context, Template};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(crate = "rocket::serde")]
pub struct ComplexTransaction {
    pub block_hash: String,
    pub block_number: String,
    pub from: String,
    pub gas: String,
    pub gas_price: String,
    pub hash: String,
    pub input: String,
    pub max_fee_per_gas: String,
    pub max_priority_fee_per_gas: String,
    pub nonce: String,
    pub r: String,
    pub s: String,
    pub to: String,
    pub transaction_index: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub v: String,
    pub value: String,
}

#[get("/transaction/<tx_hash>")]
pub async fn transaction(tx_hash: &str) -> Template {
    let t_h = crate::clean(tx_hash.to_string());
    let transaction = &parser::parse_request(
        "eth",
        "transaction",
        RequestData {
            data: serde_json::json!({ "tx": t_h }),
        },
    )
    .await;

    let result = transaction.data["transaction"].clone();

    Template::render("transaction", context! { transaction: result })
}
