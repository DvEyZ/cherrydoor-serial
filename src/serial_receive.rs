use std::{io::ErrorKind, sync::Arc, pin::Pin};

use cherrydoor_command::Heartbeat;
use futures::{lock::Mutex, StreamExt, SinkExt};
use futures::channel::mpsc::{Sender, Receiver, self};
use tokio::io::{AsyncRead, AsyncReadExt};
use crate::{errors::ErrorReporter, access::AccessManager};

#[derive(Clone)]
pub struct SerialReceiveQueue {
    serial :Arc<Mutex<Pin<Box<dyn AsyncRead + Send + Sync>>>>,
    error_reporter :ErrorReporter,
    access_manager :AccessManager,
    next_taken :Arc<Mutex<bool>>,
    take_sender :Arc<Mutex<Sender<String>>>,
    take_receiver :Arc<Mutex<Receiver<String>>>
}

impl SerialReceiveQueue {
    pub fn new(serial :Arc<Mutex<Pin<Box<dyn AsyncRead + Send + Sync>>>>, error_reporter :ErrorReporter, access_manager :AccessManager) -> Self {
        let (cx, rx) = mpsc::channel(8);

        Self {
            serial, error_reporter, access_manager,
            next_taken: Arc::new(Mutex::new(false)),
            take_sender: Arc::new(Mutex::new(cx)),
            take_receiver: Arc::new(Mutex::new(rx))
        }
    }

    async fn request_profile(&self) {
        if let Err(e) = self.access_manager.request_profile().await {

        }
    }

    async fn access(&self, code :&str) {
        if let Err(e) = self.access_manager.access(code).await {
            
        };
    }

    pub async fn take_next_code(&self) -> String {
        let mut nt = self.next_taken.lock().await;
        let mut rx = self.take_receiver.lock().await;
        *nt = true;
        drop(nt);

        let code = rx.select_next_some().await;
        
        code
    }

    pub async fn run(&mut self) -> ! {
        let mut serial = self.serial.lock().await;

        loop {
            let mut buf :Vec<u8> = vec![0; 64];

            if let Err(e) = serial.read(&mut buf).await {
                match e.kind() {
                    ErrorKind::TimedOut => {
                        self.error_reporter.report_error(&Heartbeat::new().set_connection_timeout());
                    },
                    ErrorKind::BrokenPipe => {
                        self.error_reporter.report_error(&Heartbeat::new().set_connection_broken());
                    },
                    e => panic!("{}", e)
                }
            } else {
                let buf_str = String::from_utf8(buf).unwrap();

                if buf_str.starts_with("System initialized") {
                    self.request_profile().await;

                    continue;
                }

                let maybe_heartbeat = Heartbeat::from_heartbeat(buf_str);

                if let Ok(heartbeat) = maybe_heartbeat {
                    if heartbeat.all_ok() {
                        self.error_reporter.clear_error()
                    } else {
                        self.error_reporter.report_error(&heartbeat);
                    }

                    if let Some(code) = heartbeat.code {
                        let mut cx = self.take_sender.lock().await;
                        let mut nt = self.next_taken.lock().await;
                        if *nt {
                            *nt = false;
                            cx.send(code).await.unwrap();
                            drop(cx)
                        } else {
                            self.access(&code).await;
                        }
                    }
                } else {
                    self.error_reporter.report_error(&Heartbeat::new()
                        .set_invalid_heartbeat()
                    )  
                }
            }
        }
    }
}