use cucumber::World;

// Import modules
mod step_definitions;
mod support;

use support::world::CryptofolioWorld;

#[tokio::main]
async fn main() {
    CryptofolioWorld::cucumber()
        .max_concurrent_scenarios(1)
        .filter_run("tests/features", |f, _, sc| {
            let is_wip = f.tags.iter().any(|t| t == "wip") || sc.tags.iter().any(|t| t == "wip");
            !is_wip
        })
        .await;
}
