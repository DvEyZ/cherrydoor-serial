use std::{fmt::Display, error::Error, sync::Arc};

use cherrydoor_command::Command;
use reqwest::StatusCode;
use serde::{Serialize, Deserialize};

use crate::serial_send::SerialSendQueue;

pub enum AccessStatus {
    Granted, Denied
}

#[derive(Serialize)]
pub struct AccessCodeAccess<'a> {
    pub code :&'a str,
}

#[derive(Clone)]
pub struct AccessManager {
    api_url :Arc<String>,
    api_token :Arc<String>,
    send :SerialSendQueue
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessError {
    message :String
}

#[derive(Serialize, Deserialize)]
struct ActiveAccessProfile {
    name :String
}

impl Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for AccessError {}

impl AccessManager {
    pub fn new(send :SerialSendQueue, api_url :String, api_token :String) -> Self {
        Self {
            send,
            api_url: Arc::new(api_url),
            api_token :Arc::new(api_token)
        }
    }

    pub async fn request_profile(&self) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();

        let prof = client.get(format!("{}/active-profile", self.api_url))
            .bearer_auth(&self.api_token)
            .send().await?
        .json::<ActiveAccessProfile>().await?;

        let res = client.post(format!("{}/active-profile", self.api_url))
            .bearer_auth(&self.api_token)
            .json(&ActiveAccessProfile {
                name: prof.name
            })
        .send().await?;

        match res.error_for_status() {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e))
        }
    }

    pub async fn access(&self, code :&str) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();

        let res = client.post(format!("{}/access/code", self.api_url))
            .json(&AccessCodeAccess {
                code
            })
        .send().await?;

        let result = match res.status() {
            StatusCode::NO_CONTENT => AccessStatus::Granted,
            StatusCode::NOT_FOUND 
            | StatusCode::BAD_REQUEST => AccessStatus::Denied,
            _ => return Err(Box::new(
                res.json::<AccessError>().await.unwrap_or(AccessError { message: "unexpected response".to_string() })
            ))
        };

        let command = match result {
            AccessStatus::Granted => {
                Command::new()
                    .open_for(10)
                    .display_text_for("Wejdz".to_string(), 10)
                    .play_sound(1)
            },
            AccessStatus::Denied => {
                Command::new()
                    .display_text_for("Odmowa dostepu".to_string(), 10)
                    .play_sound(2)
            }
        };

        self.send.push_command(command).await?;

        Ok(())
    }
}