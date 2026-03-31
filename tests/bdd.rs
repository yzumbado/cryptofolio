use cucumber::World;

// Import modules
mod step_definitions;
mod support;

use support::world::CryptofolioWorld;

#[tokio::main]
async fn main() {
    CryptofolioWorld::cucumber().run("tests/features").await;
}
