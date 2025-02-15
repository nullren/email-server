use std::future::Future;
use std::pin::Pin;
use crate::socket::{SocketError, SocketHandler};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct SmtpServer {}

impl SocketHandler for SmtpServer {
    type Future = Pin<Box<dyn Future<Output = Result<(), SocketError>> + Send>>;

    fn handle_connection(&mut self, _stream: TcpStream) -> Self::Future {
        Box::pin(async move { Ok(())})
    }
}
