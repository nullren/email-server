use derive_builder::Builder;

#[derive(Default, Builder, Debug, Clone)]
pub struct Message {
    pub sender_domain: String,
    pub from: String,
    pub to: Vec<String>,
    pub data: Vec<u8>,
}