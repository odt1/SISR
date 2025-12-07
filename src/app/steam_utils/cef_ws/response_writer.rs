use serde::Serialize;
use std::sync::{Arc, Mutex};

pub struct ResponseWriter {
    response: Arc<Mutex<Option<serde_json::Value>>>,
}

impl ResponseWriter {
    pub fn new() -> Self {
        Self {
            response: Arc::new(Mutex::new(None)),
        }
    }
    pub fn write<T: Serialize>(&self, data: T) -> Result<(), String> {
        match serde_json::to_value(data) {
            Ok(value) => {
                if let Ok(mut response) = self.response.lock() {
                    *response = Some(value);
                    Ok(())
                } else {
                    Err("Failed to acquire response lock".to_string())
                }
            }
            Err(e) => Err(format!("Failed to serialize response: {}", e)),
        }
    }

    pub fn write_message(&self, message: impl Into<String>) -> Result<(), String> {
        self.write(serde_json::json!({ "message": message.into() }))
    }

    pub(crate) fn take_response(&self) -> Option<serde_json::Value> {
        if let Ok(mut response) = self.response.lock() {
            response.take()
        } else {
            None
        }
    }
}

impl Default for ResponseWriter {
    fn default() -> Self {
        Self::new()
    }
}
