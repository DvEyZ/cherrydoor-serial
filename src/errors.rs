use std::{error::Error, sync::Arc};
use async_std::fs::{File, OpenOptions};
use cherrydoor_command::Heartbeat;
use futures::{lock::Mutex, AsyncWriteExt};
use crate::access::AccessStatus;

#[derive(Clone)]
pub struct ErrorReporter {
    log_file :Arc<Mutex<File>>
}

impl ErrorReporter {
    pub async fn new(log_file_name :&str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            log_file: Arc::new(Mutex::new(
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(log_file_name)
                .await?
            ))
        })
    }

    #[allow(unused_variables)]
    pub fn report_error(&self, e :&Heartbeat) {
        // Write an error to the database
    }

    pub async fn report_access(&self, code :String, status :AccessStatus) {
        let mut log_file = self.log_file.lock().await;

        let line = format!("{} {}\n", code, match status {
            AccessStatus::Granted => "granted",
            AccessStatus::Denied => "denied"
        });

        log_file.write_all(line.as_bytes()).await.unwrap();
        log_file.flush().await.unwrap();
    }

    pub fn clear_error(&self) {
        // Mark the last error as "resolved"
    }
}