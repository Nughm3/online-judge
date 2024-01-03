use std::env;

use color_eyre::Result;
use online_judge::web;
use tracing_subscriber::{prelude::*, EnvFilter};
use tracing_tree::HierarchicalLayer;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(HierarchicalLayer::new(2))
        .try_init()?;

    let server_address = env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| String::from("0.0.0.0:80"))
        .parse()?;

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| String::from("sqlite://judge.db"));

    let contest_dir = env::var("CONTEST_DIR")
        .unwrap_or_else(|_| String::from("contests"))
        .into();

    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| String::from("static"));

    let judge_config_path = env::var("JUDGE_CONFIG")
        .unwrap_or_else(|_| String::from("judge.toml"))
        .into();

    let config = web::Config {
        server_address,
        database_url,
        contest_dir,
        static_dir,
        judge_config_path,
    };

    web::serve(config).await.map_err(|e| e.into_report())?;

    Ok(())
}

// fn testing() -> Result<()> {
//     use online_judge::*;
//     let config = std::fs::read_to_string("judge.toml")?;
//     let config = toml::from_str(&config)?;
//     let contest = contest::Contest::load("/home/isaac/judge-data/contests/CSES")?;
//     let task = &contest.tasks[0];
//     let code = include_str!("../test.cpp").to_string();
//     let submission = judge::Submission {
//         code,
//         language: "C++ 17".into(),
//     };
//     let result = judge::run(&config, submission, &task, contest.rlimits)?;
//     tracing::info!("{result:#?}");
//     Ok(())
// }
