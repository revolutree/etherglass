use crate::{json_value_hex_to_int, parser, RequestData};
use rocket::serde::Serialize;
use rocket_dyn_templates::{context, Template};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(crate = "rocket::serde")]
pub struct SimpleAddress {
    pub address: String,
    pub balance: String,
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
