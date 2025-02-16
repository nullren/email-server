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
                if state.state != SmtpState::Data && buf.starts_with(b"QUIT") {
                    stream.write_all(b"221 Goodbye\r\n").await?;
                    break;
                }
                match state.read(&buf[..n]) {
                    Ok(Some(output)) => {
                        stream
                            .write_all(format!("{}\r\n", output).as_bytes())
                            .await?;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        stream.write_all(format!("{}\r\n", e).as_bytes()).await?;
                    }
                }
                if state.state == SmtpState::Done {
                    // self.message_queue.push(state.message);
                    state = SmtpMessageReader::default();
                }
            }
            Ok(())
        })
    }
}
