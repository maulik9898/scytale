#![allow(unused_variables)]
#![allow(unused_imports)]

use std::net::SocketAddr;

use scytale::serve;

#[tokio::main]
async fn main() {

    dotenv::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let secret = std::env::var("JWT_SECRET").expect("SECRET must be set");
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    serve(addr, db_url.as_str(), secret.as_str()).await;
    
    
}


