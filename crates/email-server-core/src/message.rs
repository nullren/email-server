use async_trait::async_trait;
use derive_builder::Builder;
use std::error::Error;

#[derive(Default, Builder, Debug, Clone)]
pub struct Message {
    pub sender_domain: String,
    pub from: String,
    pub to: Vec<String>,
    pub data: Vec<u8>,
}

#[async_trait]
pub trait Handler {
    async fn handle_message(&self, message: Message) -> Result<(), Box<dyn Error>>;
}

pub struct PrintHandler;

#[async_trait]
impl Handler for PrintHandler {
    async fn handle_message(&self, message: Message) -> Result<(), Box<dyn Error>> {
        tracing::debug!(
            "Received message from {} to {} with {} bytes of data",
            message.from,
            message.to.join(", "),
            message.data.len()
        );
        Ok(())
    }
}

pub struct MultiHandler {
    handlers: Vec<Box<dyn Handler + Send + Sync>>,
}

pub fn multi_handler(handlers: Vec<Box<dyn Handler + Send + Sync>>) -> MultiHandler {
    MultiHandler { handlers }
}

#[async_trait]
impl Handler for MultiHandler {
    async fn handle_message(&self, message: Message) -> Result<(), Box<dyn Error>> {
        for handler in &self.handlers {
            handler.handle_message(message.clone()).await?;
        }
        Ok(())
    }
}
