use std::str::FromStr;

use chrono::{offset::Utc, Duration};
use graphile_worker::{task, JobSpec, WorkerContext, WorkerOptions};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnectOptions;
use tracing_subscriber::{
    filter::EnvFilter, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

#[derive(Deserialize, Serialize)]
struct HelloPayload {
    message: String,
}

#[task]
async fn say_hello(payload: HelloPayload, _ctx: WorkerContext) -> Result<(), String> {
    println!("Hello {} !", payload.message);
    // 30% chance to succeed to test retry
    let mut rng = rand::thread_rng();
    if rng.gen_range(0..100) < 70 {
        return Err("Failed".to_string());
    }
    Ok(())
}

fn enable_logs() {
    let fmt_layer = tracing_subscriber::fmt::layer();
    // Log level set to debug except for sqlx set at warn (to not show all sql requests)
    let filter_layer = EnvFilter::try_new("debug,sqlx=warn").unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

#[tokio::main]
async fn main() {
    enable_logs();

    let pg_options = PgConnectOptions::from_str("postgres://postgres:root@localhost:5432").unwrap();

    let pg_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect_with(pg_options)
        .await
        .unwrap();

    let worker = WorkerOptions::default()
        .concurrency(2)
        .schema("example_simple_worker")
        .define_job(say_hello)
        .pg_pool(pg_pool)
        .init()
        .await
        .unwrap();

    let helpers = worker.create_utils();

    helpers
        .add_job::<say_hello>(
            HelloPayload {
                message: "world".to_string(),
            },
            Some(JobSpec {
                // Now + 10 seconds
                run_at: Some(Utc::now() + Duration::seconds(10)),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    worker.run().await.unwrap();
}
