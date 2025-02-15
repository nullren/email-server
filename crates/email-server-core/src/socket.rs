use std::{
    error::Error,
    fmt::{Display, Formatter},
    future::Future,
};

use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use tokio::time::Instant;

#[derive(Debug)]
pub enum SocketError {
    BindFailed(std::io::Error),
    ConnectionFailed(std::io::Error),
    IoError(std::io::Error),
    Closed,
}

impl Display for SocketError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            SocketError::BindFailed(e) => write!(f, "BindFailed: {}", e),
            SocketError::ConnectionFailed(e) => write!(f, "ConnectionFailed: {}", e),
            SocketError::IoError(e) => write!(f, "IoError: {}", e),
            SocketError::Closed => write!(f, "Closed"),
        }
    }
}

impl Error for SocketError {}

impl From<std::io::Error> for SocketError {
    fn from(e: std::io::Error) -> Self {
        SocketError::IoError(e)
    }
}

pub trait SocketHandler {
    type Future: Future<Output = Result<(), SocketError>>;
    fn handle_connection(&mut self, stream: TcpStream) -> Self::Future;
}

pub async fn run<H>(addr: &str, handler: H) -> Result<(), SocketError>
where
    H: SocketHandler + Clone + Send + 'static,
    <H as SocketHandler>::Future: Send,
{
    let listener = TcpListener::bind(addr)
        .await
        .map_err(SocketError::BindFailed)?;
    println!("SMTP Server listening on {}", addr);

    loop {
        let (socket, addr) = listener
            .accept()
            .await
            .map_err(SocketError::ConnectionFailed)?;
        println!("New connection: {}", addr);

        let mut handler = handler.clone();
        let start = Instant::now();

        task::spawn(async move {
            match handler.handle_connection(socket).await {
                Ok(_) => {}
                Err(e) => eprintln!("Error: {}", e),
            }
            println!("Connection closed in {} ms", start.elapsed().as_millis());
        });
    }
}
