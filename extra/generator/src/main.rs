mod domain;
mod generate;
mod serde_macro;

use crate::domain::Config;
use crate::generate::generate_events;
use common_logging::init::init_regular_logging;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    init_regular_logging();
    let exit = signal::ctrl_c();

    let config_str = std::fs::read_to_string("extra/generator/seed.yaml")?;
    let config: Config = serde_yaml::from_str(&config_str)?;

    let service = generate_events(&config);

    tokio::select! {
        _ = service => {},
        _ = exit => {
              log::info!("Interrupted");
        }
    };

    Ok(())
}
