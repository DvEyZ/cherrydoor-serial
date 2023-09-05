use std::{sync::Arc, error::Error, pin::Pin};

use cherrydoor_command::Command;
use futures::lock::Mutex;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone)]
pub struct SerialSendQueue {
    serial :Arc<Mutex<Pin<Box<dyn AsyncWrite + Send + Sync>>>>,
}

impl SerialSendQueue {
    pub fn new(serial :Arc<Mutex<Pin<Box<dyn AsyncWrite + Send + Sync>>>>) -> Self {
        Self { 
            serial
        }
    }


    pub async fn push_command(&self, command :Command) -> Result<(), Box<dyn Error>>{
        let mut serial = self.serial.lock().await;
        let cmd = command.into_string()?;

        serial.write(format!("{}\n", cmd).as_bytes()).await?;

        Ok(())
    }
}