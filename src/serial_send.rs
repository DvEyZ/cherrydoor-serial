use std::{sync::Arc, error::Error};

use cherrydoor_command::Command;
use futures::lock::Mutex;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone)]
pub struct SerialSendQueue {
    serial :Arc<Mutex<dyn AsyncWrite + Send + Sync + Unpin>>,
}

impl SerialSendQueue {
    pub fn new(serial :Arc<Mutex<dyn AsyncWrite + Send + Sync + Unpin>>) -> Self {
        Self { 
            serial
        }
    }


    pub async fn push_command(&self, command :Command) -> Result<(), Box<dyn Error + Send + Sync>>{
        let mut serial = self.serial.lock().await;
        let cmd = command.into_string()?;

        serial.write(format!("{}\n", cmd).as_bytes()).await?;

        Ok(())
    }
}