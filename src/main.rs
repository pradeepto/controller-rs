#![allow(unused_imports, unused_variables)]
use std::env;
use log::{info, warn, error, debug, trace};
use prometheus::{TextEncoder, Encoder};
pub use controller::*;

use actix_web::{
    web::{self, Data},
    HttpRequest, HttpResponse, middleware
};
use actix_web::{get, App, HttpServer, Responder};

#[get("/metrics")]
async fn metrics(c: Data<Controller>, _req: HttpRequest) -> impl Responder {
    let metrics = c.metrics();
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder.encode(&metrics, &mut buffer).unwrap();
    HttpResponse::Ok().body(buffer)
}

#[get("/health")]
async fn health(_: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json("healthy")
}

#[get("/")]
async fn index(c: Data<Controller>, _req: HttpRequest) -> impl Responder {
    let state = c.state().unwrap();
    HttpResponse::Ok().json(state)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Logging
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "actix_web=info,controller=info,kube=debug");
    }
    env_logger::init();

    // Set up kube access + fetch initial state. Crashing on failure here.
    let cfg = if let Ok(c) = kube::config::incluster_config() {
        c
    } else {
        kube::config::load_kube_config().await.expect("Failed to load kube config")
    };
    let c = state::init(cfg).await.expect("Failed to initialize controller");

    HttpServer::new(move || {
        App::new()
            .data(c.clone())
            .wrap(middleware::Logger::default()
                .exclude("/health")
            )
            .service(index)
            .service(health)
            .service(metrics)
        })
        .bind("0.0.0.0:8080").expect("Can not bind to 0.0.0.0:8080")
        .shutdown_timeout(0) // example server
        .start()
        .await
}
