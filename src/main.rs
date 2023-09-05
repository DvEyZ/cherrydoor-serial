mod serial_send;
mod serial_receive;
mod errors;
mod access;

use std::{sync::Arc, time::Duration};
use access::AccessManager;
use axum::{Router, Json, http::StatusCode, routing::{get, post}, extract::State};
use cherrydoor_command::Command;
use errors::ErrorReporter;
use futures::{try_join, lock::Mutex};
use serial_receive::SerialReceiveQueue;
use serial_send::SerialSendQueue;
#[allow(unused_imports)]
use tokio_serial::{SerialPortBuilderExt, SerialPort};
use tokio::io::{AsyncRead, AsyncWrite};
use std::pin::Pin;


#[derive(Clone)]
pub struct AppState {
    pub send_queue :SerialSendQueue,
    pub recv_queue :SerialReceiveQueue
}

#[cfg(not(debug_assertions))]
fn get_io() -> (Box<dyn AsyncRead + Send + Sync>, Box<dyn AsyncWrite + Send + Sync>) {
    let serial = std::env::var("SERIAL_PORT_PATH").unwrap();
    let baud_rate = std::env::var("SERIAL_PORT_BAUD_RATE").unwrap().parse::<u32>().unwrap();

    let serial_port = Box::new(tokio_serial::new(serial, baud_rate)
        .timeout(Duration::from_secs(5))
    .open_native_async().unwrap());

    (serial_port, serial_port)
}

#[cfg(debug_assertions)]
fn get_io() -> (Pin<Box<dyn AsyncRead + Send + Sync>>, Pin<Box<dyn AsyncWrite + Send + Sync>>)  {
    (Box::pin(tokio::io::stdin()), Box::pin(tokio::io::stdout()))
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let (input, output) = get_io();
    let api_url = std::env::var("API_URL").unwrap();
    let api_token = std::env::var("API_TOKEN").unwrap();

    let error_reporter = ErrorReporter::new();
    let send_queue = SerialSendQueue::new(Arc::new(Mutex::new(output)));
    let access_manager = AccessManager::new(send_queue.clone(), api_url, api_token);
    let mut recv_queue = SerialReceiveQueue::new(
        Arc::new(Mutex::new(input)), 
        error_reporter,
        access_manager
    );

    let app_state = AppState {
        send_queue: send_queue.clone(),
        recv_queue: recv_queue.clone()
    };

    let app = Router::new()
        .route("/", post(execute))
        .route("/register", get(register))
        .with_state(app_state)
    .into_make_service();
    
    let recv_queue_task = tokio::task::spawn(async move {
        recv_queue.run().await
    });

    let server_task = tokio::task::spawn(async move {
        axum::Server::bind(&"0.0.0.0:9000".parse::<std::net::SocketAddr>().unwrap()).serve(app).await
    });

    if let Err(e) = try_join!(server_task, recv_queue_task) {
        println!("{}", e)
    };
}

async fn execute(
    State(app_state) :State<AppState>,
    Json(command) :Json<Command>
) -> (StatusCode, Result<(), String>) {
    match app_state.send_queue.push_command(command).await {
        Ok(()) => (StatusCode::NO_CONTENT, Ok(())),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Err(format!("{}", e)))
    }
}

#[axum::debug_handler]
async fn register(
    State(app_state) :State<AppState>
) -> (StatusCode, String) {

    let Ok(code) = tokio::time::timeout(
        Duration::from_secs(10), 
        app_state.recv_queue.take_next_code()
    ).await else {
        return (StatusCode::NOT_FOUND, String::from("timed out"))
    };

    (StatusCode::OK, code)
}