use crate::socket::{SocketError, SocketHandler};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub mod command;
pub mod state;

use state::{SmtpMessageReader, SmtpState};

#[derive(Clone)]
pub struct SmtpServer {}

impl SocketHandler for SmtpServer {
    type Future = Pin<Box<dyn Future<Output = Result<(), SocketError>> + Send>>;

    fn handle_connection(&mut self, mut stream: TcpStream) -> Self::Future {
        Box::pin(async move {
            stream
                .write_all(b"220 Welcome to the SMTP server\r\n")
                .await?;
            let mut state = SmtpMessageReader::default();
            loop {
                let mut buf = [0; 1024];
                let n = stream.read(&mut buf).await?;
                if n == 0 {
                    break;
                }

                match state.state {
                    SmtpState::Data => {
                        // Handle data collection
                        if let Some(response) = handle_data(&mut state, &buf[..n]) {
                            stream
                                .write_all(format!("{}\r\n", response).as_bytes())
                                .await?;
                        }
                    }
                    _ => {
                        // Handle command parsing
                        if let Some(response) = handle_command(&mut state, &buf[..n]) {
                            stream
                                .write_all(format!("{}\r\n", response).as_bytes())
                                .await?;
                        }
                    }
                }

                if state.state == SmtpState::Done {
                    // Process the completed message
                    // self.message_queue.push(state.message);
                    state = SmtpMessageReader::default();
                }
            }
            Ok(())
        })
    }
}

fn handle_command(state: &mut SmtpMessageReader, line: &[u8]) -> Option<&'static str> {
    if line.starts_with(b"QUIT") {
        return Some("221 Goodbye");
    }

    match state.read(line) {
        Ok(Some(output)) => Some(output),
        Ok(None) => None,
        Err(e) => Some(e),
    }
}

fn handle_data(state: &mut SmtpMessageReader, line: &[u8]) -> Option<&'static str> {
    if line == b".\r\n" {
        state.state = SmtpState::Done;
        Some("250 Mail Delivered")
    } else {
        if line.starts_with(b"..") {
            state.message.data.extend_from_slice(&line[1..]);
        } else {
            state.message.data.extend_from_slice(line);
        }
        None
    }
}
