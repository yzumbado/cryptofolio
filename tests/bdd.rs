use cucumber::World;

// Import modules
mod step_definitions;
mod support;

use support::world::CryptofolioWorld;

#[tokio::main]
async fn main() {
    CryptofolioWorld::cucumber()
        .filter_run("tests/features", |_, _, sc| {
            !sc.tags.iter().any(|t| t == "wip")
        })
        .await;
}
