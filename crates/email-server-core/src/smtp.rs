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
                let command = SmtpCommand::from_bytes(&buf[..n]);
                println!("Received {:?}", command);
                if n == 0 {
                    break;
                }
                stream.write_all(b"503 Bad sequence of commands\r\n").await?;
            }
            Ok(())
        })
    }
}

#[derive(Debug)]
pub enum SmtpCommand {
    Unknown,
    Helo(String),
    MailFrom(String),
    RcptTo(String),
    Data(Vec<u8>),
}

impl SmtpCommand {
    pub fn from_bytes(bytes: &[u8]) -> SmtpCommand {
        let mut parts = bytes.split(|&b| b == b' ');
        match parts.next() {
            Some(b"HELO") => {
                let domain = parts.next().unwrap_or(&[]).to_vec();
                SmtpCommand::Helo(String::from_utf8_lossy(&domain).to_string())
            }
            Some(b"MAIL") => {
                let from = parts.next().unwrap_or(&[]).to_vec();
                SmtpCommand::MailFrom(String::from_utf8_lossy(&from).to_string())
            }
            Some(b"RCPT") => {
                let to = parts.next().unwrap_or(&[]).to_vec();
                SmtpCommand::RcptTo(String::from_utf8_lossy(&to).to_string())
            }
            Some(b"DATA") => {
                SmtpCommand::Data(parts.next().unwrap_or(&[]).to_vec())
            }
            _ => SmtpCommand::Unknown,
        }
    }
}
