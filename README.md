### Etherglass

Side-car explorer for JSON-RPC enabled Ethereum nodes
- no DB / on the fly rpc calls
- Optional redis cache to cache addresses, blocks, transactions for faster response time
- ENS support
- Search bar for blocks, addresses, transactions

### Run

Install Rust

```
ETH_RPC_ENDPOINT=http://localhost:8454 cargo run
```

Basic frontend available on `localhost:8000`
