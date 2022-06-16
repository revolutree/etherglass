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

// pub mod login (TODO);
pub mod pages;
pub mod parser;
pub mod rcache;
pub mod crawler;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RequestData {
    pub data: serde_json::Value,
}

use rocket_dyn_templates::Template;

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

/// This interprets JSON-RPC api methods retrieving data from the node.
/// Allowed methods are in parser::parse_request method.
#[post("/<api>/<method>", format = "application/json", data = "<option_data>")]
async fn api(
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

/// Currently unused Sign In with Ethereum endpoint (TODO)
#[post("/login", format = "application/json", data = "<data>")]
async fn web3_login(data: Json<LoginRequest>) -> Json<LoginResponse> {
    let message: siwe::Message = data.message.parse().unwrap();
    let bytes_signature = data.signature.as_bytes();

    let mut signature_bytes = [0u8; 65];
    for i in 0..65 {
        signature_bytes[i] = bytes_signature[i];
    }

    if let Err(_e) = message.verify(signature_bytes, None, None, None) {
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
/// Returns the latest pages::home::LATEST_BLOCKS_AMOUNT blocks as sse events every 10 seconds
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

use std::sync::Mutex;
use std::sync::RwLock;

lazy_static! {
    static ref IS_POLLING_EVENTS: RwLock<bool> = RwLock::new(false);
    static ref REDIS_CACHE: Mutex<bool> = Mutex::new(false);
}

const POLLING_INTERVAL : u64 = 10;

/// Receive a message from a form submission and broadcast it to any receivers.
/// Activate the SSE events
/// Retrieve the latest X blocks from the node and send them as SSE events
#[get("/start_polling")]
async fn start_polling(queue: &State<Sender<BlockEvent>>) -> Json<StatusResponse> {

    // Poll for new incoming blocks
    // Check if it's already polling in global Lazy static
    let is_polling_events = IS_POLLING_EVENTS.read().unwrap().to_owned();
    if is_polling_events {
        // Already polling
    } else {
        std::thread::spawn(move || {
            // Set global Lazy static to true
            let mut new_settings = IS_POLLING_EVENTS.write().unwrap();
            *new_settings = true;
        });
        loop { // Retrieve latest X blocks
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

            let latest_blocks: Vec<pages::home::SimpleBlock> =
                pages::home::get_latest_blocks(b_n).await;

            // Publish blocks to SSE clients
            let _ = queue.send(BlockEvent {
                blocks: latest_blocks,
            });

            // Get cache status
            let redis_cache = crate::Cache {
                enabled: *crate::REDIS_CACHE.lock().unwrap(),
        
                // Temporary redefining it here, should be moved around coming from the Rocket handler
                redis_client: Some(redis::Client::open("redis://localhost:6379").unwrap()),
            };

            if redis_cache.enabled {
                // If cache is enable, crawl the block and cache addresses with corresponding transactions
                let _ = pages::address::cache_addresses_transactions_from_block(b_n as i64).await;
            }

            std::thread::sleep(std::time::Duration::from_secs(POLLING_INTERVAL));
        }
    }

    return Json(StatusResponse { status: true });
}

pub struct Cache {
    enabled: bool,
    redis_client: Option<redis::Client>,
}

use clap::Parser;

/// CLI ARGS
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Enable redis cache
    #[clap(long)]
    cache: bool,
    /// Enable crawler of past transaction
    #[clap(long)]
    crawler: bool,
    /// Starting block for crawler
    #[clap(long)]
    start_block: Option<i64>,
    /// Ending block for crawler
    #[clap(long)]
    end_block: Option<i64>,
}

#[launch]
fn rocket() -> _ {
    let args = Args::parse();

    
    println!("Enable cache: {}!", args.cache);
    let enable_cache = args.cache;

    let mut redis_cache = false;
    if enable_cache {
        redis_cache = true
    }

    let mut static_cache = REDIS_CACHE.lock().unwrap();
    *static_cache = redis_cache;

    // Define redis client
    // If crawler is enabled, start it
    // Starting block and ending block are optional
    let redis_client: Option<redis::Client> = if redis_cache {
        if args.crawler {
            std::thread::spawn(move || {
                let starting_block = if let Some(start_block) = args.start_block {
                    start_block
                } else {
                    0
                };
                let ending_block = if let Some(end_block) = args.end_block {
                    end_block
                } else {
                    0
                };

                if ending_block <= starting_block {
                    println!("Ending block must be greater than starting block");
                    return;
                }
                async_std::task::block_on(async move {
                    crawler::crawler(starting_block,ending_block).await;
                });
            });
        }
        Some(redis::Client::open("redis://localhost:6379").unwrap())
    } else {
        None
    };

    let cache = Cache {
        enabled: redis_cache,
        redis_client: redis_client,
    };
    
    // Create a channel to send messages to the SSE clients
    let queue = channel::<BlockEvent>(1024).0;

    // Start the SSE and API server
    rocket::build()
        .manage(queue)
        .manage(cache)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![
                api,
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

// Define web3 client for node connections
pub struct EtherClient {
    web3: web3::Web3<web3::transports::Http>,
    ens: web3::contract::ens::Ens<web3::transports::Http>,
}

/// Web3 client for node connections
async fn client() -> Result<EtherClient, String> {
    let rpc_endpoint = std::env::var("ETH_RPC_ENDPOINT").map_err(|e| format!("{}", e));
    let rpc_endpoint = rpc_endpoint.unwrap_or_else(|_| "http://localhost:8545".to_string());
    let transport = web3::transports::Http::new(&rpc_endpoint).unwrap();
    let web3 = web3::Web3::new(transport.clone());
    let ens = web3::contract::ens::Ens::new(transport.clone());

    Ok(EtherClient {
        web3: web3,
        ens: ens,
    })
}

/// Normalize hex strings to lowercase and remove 0x prefix
/// TODO : This should be done by a proper library
pub fn json_value_hex_to_int(h: serde_json::Value) -> i128 {
    let input = h.as_str().unwrap().replace("0x", "").replace("\"", "");
    let mut _output = 0;
    _output = i128::from_str_radix(&input.to_string(), 16).unwrap();
    _output
}

pub fn clean(s: String) -> String {
    s.replace("\"", "")
}