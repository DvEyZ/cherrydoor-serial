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
use tokio_serial::SerialStream;
#[allow(unused_imports)]
use tokio_serial::{SerialPortBuilderExt, SerialPort};
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Clone)]
pub struct AppState {
    pub send_queue :SerialSendQueue,
    pub recv_queue :SerialReceiveQueue
}

type Serial = (Arc<Mutex<dyn AsyncRead + Send + Sync + Unpin>>, Arc<Mutex<dyn AsyncWrite + Send + Sync + Unpin>>);

trait AsyncReadWrite :AsyncRead + AsyncWrite {}
impl AsyncReadWrite for SerialStream {}

#[allow(unused)]
fn get_io_release() -> Serial {
    let serial = std::env::var("SERIAL_PORT_PATH").unwrap();
    let baud_rate = std::env::var("SERIAL_PORT_BAUD_RATE").unwrap().parse::<u32>().unwrap();

    let sp = tokio_serial::new(serial, baud_rate)
        .timeout(Duration::from_secs(5))
    .open_native_async().unwrap();

    let asp = Arc::new(Mutex::new(Box::pin(sp)));

    (asp.clone(), asp)
}

#[allow(unused)]
fn get_io_debug() -> Serial {
    (
        Arc::new(Mutex::new(Box::pin(tokio::io::stdin()))), 
        Arc::new(Mutex::new(Box::pin(tokio::io::stdout())))
    )
}

fn get_io() -> Serial {
#[cfg(debug_assertions)]
    return get_io_debug();
#[cfg(not(debug_assertions))]
    return get_io_release();    
}   

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let (input, output) = get_io();
    let api_url = std::env::var("API_URL").unwrap();
    let api_token = std::env::var("API_TOKEN").unwrap();
    let log_file_name = std::env::var("LOG_FILE").unwrap();

    let error_reporter = ErrorReporter::new(&log_file_name).await.unwrap();
    let send_queue = SerialSendQueue::new(output);
    let access_manager = AccessManager::new(send_queue.clone(), api_url, api_token);
    let mut recv_queue = SerialReceiveQueue::new(
        input, 
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
        recv_queue.run().await;
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