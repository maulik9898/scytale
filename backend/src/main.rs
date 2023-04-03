#![allow(unused_variables)]
#![allow(unused_imports)]

use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let secret = std::env::var("JWT_SECRET").expect("SECRET must be set");
    let admin_email = std::env::var("ADMIN_EMAIL").expect("ADMIN_EMAIL must be set");
    let admin_password = std::env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set");
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let admin_name = "Maulik Patel".to_string();

    let app = scytale::Scytale::new(
        addr,
        db_url,
        secret,
        admin_email,
        admin_password,
        admin_name,
    )
    .start()
    .await;
}
