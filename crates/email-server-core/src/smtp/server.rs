use crate::smtp::state;
use crate::smtp::state::Message;
use crate::socket::{SocketError, SocketHandler};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct Server {}

impl SocketHandler for Server {
    type Future = Pin<Box<dyn Future<Output = Result<(), SocketError>> + Send>>;

    fn handle_connection(&mut self, mut stream: TcpStream) -> Self::Future {
        Box::pin(async move {
            stream
                .write_all(b"220 Welcome to the SMTP server\r\n")
                .await?;
            let mut message = Message::default();
            let mut state = state::new_state();
            loop {
                let mut buf = [0; 1024];
                let n = stream.read(&mut buf).await?;
                if n == 0 {
                    break;
                }

                if !state.is_data_collect() && buf.starts_with(b"QUIT") {
                    stream.write_all(b"221 Goodbye\r\n").await?;
                    break;
                }

                match state.process_line(&buf[..n], &mut message) {
                    (Some(output), Some(next_state)) => {
                        stream
                            .write_all(format!("{}\r\n", output).as_bytes())
                            .await?;
                        state = next_state;
                    }
                    (Some(output), None) => {
                        stream
                            .write_all(format!("{}\r\n", output).as_bytes())
                            .await?;
                        break;
                    }
                    (None, _) => {}
                }

                if state.is_done() {
                    // TODO: Handle the message
                    message = Message::default();
                    state = state::new_state();
                }
            }
            Ok(())
        })
    }
}
