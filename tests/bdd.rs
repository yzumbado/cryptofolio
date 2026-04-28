use cucumber::World;

// Import modules
mod step_definitions;
mod support;

use support::world::CryptofolioWorld;

#[tokio::main]
async fn main() {
    CryptofolioWorld::cucumber()
        .filter_run("tests/features", |f, _, sc| {
            let is_wip = f.tags.iter().any(|t| t == "wip") || sc.tags.iter().any(|t| t == "wip");
            !is_wip
        })
        .await;
}
