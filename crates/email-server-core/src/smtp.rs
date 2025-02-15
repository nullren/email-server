use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::socket::{SocketError, SocketHandler};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct SmtpServer {}

impl SocketHandler for SmtpServer {
    type Future = Pin<Box<dyn Future<Output = Result<(), SocketError>> + Send>>;

    fn handle_connection(&mut self, mut stream: TcpStream) -> Self::Future {
        Box::pin(async move {
            stream.write_all(b"220 Welcome to the SMTP server\r\n").await?;
            loop {
                let mut buf = [0; 1024];
                let n = stream.read(&mut buf).await?;
                println!("Received {}", String::from_utf8_lossy(&buf));
                if n == 0 {
                    break;
                }
                stream.write_all(b"503 Bad sequence of commands\r\n").await?;
            }
            Ok(())
        })
    }
}
