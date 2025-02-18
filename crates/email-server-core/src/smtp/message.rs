use std::error::Error;
use async_trait::async_trait;
use derive_builder::Builder;

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
        println!("Received message from {} to {} with {} bytes of data",
                 message.from,
                 message.to.join(", "),
                 message.data.len());
        Ok(())
    }
}