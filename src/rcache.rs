use redis::Commands;

pub fn set(client: redis::Client, key: &String, value: &String) -> redis::RedisResult<isize> {
    println!("Caching {}", key);
    let mut con = client.get_connection()?;
    // throw away the result, just make sure it does not fail
    let _: () = con.set(key, value)?;
    // read back the key and return it.  Because the return value
    // from the function is a result for integer this will automatically
    // convert into one.
    con.get(key)
}

pub fn get(client: redis::Client, key: &String) -> redis::RedisResult<String> {
    println!("Retrieving {}", key);
    let mut con = client.get_connection()?;
    con.get(key)
}

pub fn check_cache(client: redis::Client, key: &String) -> redis::RedisResult<bool> {
    let mut con = client.get_connection()?;
    con.exists(key)
}
