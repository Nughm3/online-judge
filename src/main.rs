use std::env;

use color_eyre::Result;
use online_judge::web;
use pico_args::Arguments;
use tracing_subscriber::{prelude::*, EnvFilter};
use tracing_tree::HierarchicalLayer;

const HELP: &str = "\
Online Judge

USAGE:
  online-judge [OPTIONS]

FLAGS:
  -h, --help          Display help information

OPTIONS:
  -a, --address       Set server address (0.0.0.0:80)
  -d, --database-url  Set database URL (sqlite://judge.db)
  -C, --contest-dir   Set contest directory (contests)
  -s, --static-dir    Set static directory (static)
  -c, --config        Set judge config path (judge.toml)
";

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(HierarchicalLayer::new(2))
        .try_init()?;

    let mut args = Arguments::from_env();

    if args.contains(["-h", "--help"]) {
        println!("{HELP}");
    } else {
        let config = web::Config {
            server_address: args
                .opt_value_from_str(["-a", "--address"])?
                .or_else(|| env::var("SERVER_ADDRESS").ok())
                .unwrap_or_else(|| String::from("0.0.0.0:80"))
                .parse()?,
            database_url: args
                .opt_value_from_str(["-d", "--database-url"])?
                .or_else(|| env::var("DATABASE_URL").ok())
                .unwrap_or_else(|| String::from("sqlite://judge.db")),
            contest_dir: args
                .opt_value_from_str(["-C", "--contest-dir"])?
                .unwrap_or_else(|| String::from("contests"))
                .into(),
            static_dir: args
                .opt_value_from_str(["-s", "--static-dir"])?
                .unwrap_or_else(|| String::from("static")),
            judge_config_path: args
                .opt_value_from_str(["-c", "--config"])?
                .unwrap_or_else(|| String::from("judge.toml"))
                .into(),
        };

        tracing::info!("starting server with config: {config:#?}");
        web::serve(config).await.map_err(|e| e.into_report())?;
    }

    Ok(())
}
