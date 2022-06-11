#[macro_use]
extern crate rocket;
#[macro_use]
extern crate lazy_static;

use rocket::response::stream::{Event, EventStream};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::tokio::select;
use rocket::tokio::sync::broadcast::{channel, error::RecvError, Sender};
use std::i64;
use web3::api::Namespace;

pub mod login;
pub mod pages;
pub mod parser;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RequestData {
    pub data: serde_json::Value,
}

use rocket_dyn_templates::Template;

pub fn json_value_hex_to_int(h: serde_json::Value) -> i128 {
    let input = h.as_str().unwrap().replace("0x", "").replace("\"", "");
    let mut _output = 0;
    _output = i128::from_str_radix(&input.to_string(), 16).unwrap();
    _output
}

pub fn clean(s: String) -> String {
    s.replace("\"", "")
}

#[post("/<api>/<method>", format = "application/json", data = "<option_data>")]
async fn hello(
    api: &str,
    method: &str,
    option_data: Option<Json<RequestData>>,
) -> Json<parser::ResponseData> {
    let mut data = RequestData {
        data: serde_json::json!({}),
    };

    if let Some(option_data) = option_data {
        data = option_data.into_inner();
    }

    let result = parser::parse_request(api, method, data).await;

    Json(result)
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginRequest {
    pub signature: String,
    pub message: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginResponse {
    pub address: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct StatusResponse {
    pub status: bool,
}

#[post("/login", format = "application/json", data = "<data>")]
async fn web3_login(data: Json<LoginRequest>) -> Json<LoginResponse> {
    let message: siwe::Message = data.message.parse().unwrap();
    let bytes_signature = data.signature.as_bytes();

    let mut signature_bytes = [0u8; 65];
    for i in 0..65 {
        signature_bytes[i] = bytes_signature[i];
    }

    if let Err(e) = message.verify(signature_bytes, None, None, None) {
        // message cannot be correctly authenticated at this time
        return Json(LoginResponse {
            address: String::from("0x0"),
        });
    }

    Json(LoginResponse {
        address: format!("{:x?}", message.address),
    })
}

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, UriDisplayQuery))]
#[serde(crate = "rocket::serde")]
struct BlockEvent {
    pub blocks: Vec<pages::home::SimpleBlock>,
}

use rocket::{Shutdown, State};

/// Returns an infinite stream of server-sent events. Each event is a message
/// pulled from a broadcast queue sent by the `post` handler.
#[get("/latest_blocks")]
async fn latest_blocks(queue: &State<Sender<BlockEvent>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}

use std::sync::RwLock;

lazy_static! {
    static ref IS_POLLING_EVENTS: RwLock<bool> = RwLock::new(false); 
}

/// Receive a message from a form submission and broadcast it to any receivers.
#[get("/start_polling")]
async fn start_polling(queue: &State<Sender<BlockEvent>>) -> Json<StatusResponse> {

    let is_polling_events = IS_POLLING_EVENTS.read().unwrap().to_owned();
    if is_polling_events {
        
    }else{
        std::thread::spawn(move||{
            let mut new_settings = IS_POLLING_EVENTS.write().unwrap();
            *new_settings = true;
        });
        loop {

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
        
            
            let latest_blocks: Vec<pages::home::SimpleBlock> = pages::home::get_latest_blocks(b_n).await;

            let _ = queue.send(BlockEvent { blocks: latest_blocks });
            // sleep 10 seconds
            std::thread::sleep(std::time::Duration::from_secs(10));
        }  
    }

    return Json(StatusResponse{
        status: true
    });
    
}

#[launch]
fn rocket() -> _ {
    let queue = channel::<BlockEvent>(1024).0;
    rocket::build()
        .manage(queue)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![
                hello,
                pages::home::index,
                pages::block::block,
                pages::block::block_hash,
                pages::transaction::transaction,
                pages::address::address,
                web3_login,
                latest_blocks,
                start_polling
            ],
        )
}

pub struct EtherClient {
    web3: web3::Web3<web3::transports::Http>,
    ens: web3::contract::ens::Ens<web3::transports::Http>,
}

async fn client() -> Result<EtherClient, String> {
    let rpc_endpoint = std::env::var("ETH_RPC_ENDPOINT")
        .map_err(|e| format!("{}", e));
    let rpc_endpoint = rpc_endpoint.unwrap_or_else(|_| "http://localhost:8545".to_string());
    let transport = web3::transports::Http::new(&rpc_endpoint).unwrap();
    let web3 = web3::Web3::new(transport.clone());
    let ens = web3::contract::ens::Ens::new(transport.clone());

    Ok(EtherClient {
        web3: web3,
        ens: ens,
    })
}
