use crate::smtp::state;
use crate::smtp::state::Message;
use crate::socket::{SocketError, SocketHandler};
use bytes::BytesMut;
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
            let mut buffer = BytesMut::with_capacity(4096);

            'outer: loop {
                let mut temp = [0; 1024];
                let n = stream.read(&mut temp).await?;
                if n == 0 {
                    break 'outer;
                }
                buffer.extend_from_slice(&temp[..n]);

                while let Some(pos) = buffer.windows(2).position(|x| x == b"\r\n") {
                    let buf = buffer.split_to(pos + 2);
                    if !state.is_data_collect() && buf.starts_with(b"QUIT") {
                        stream.write_all(b"221 Goodbye\r\n").await?;
                        break 'outer;
                    }

                    match state.process_line(&buf, &mut message) {
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
                            break 'outer;
                        }
                        (None, _) => {}
                    }

                    if state.is_done() {
                        println!("Message received: {:?}", message);
                        // TODO: Handle the message
                        message = Message::default();
                        state = state::new_state();
                    }
                }
            }
            Ok(())
        })
    }
}
