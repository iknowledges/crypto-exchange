use std::sync::LazyLock;

use redis::Client;

static REDIS_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    Client::open(redis_url).expect("Invalid Redis URL")
});

pub fn get_client() -> &'static Client {
    &REDIS_CLIENT
}