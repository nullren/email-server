use async_trait::async_trait;
use std::pin::Pin;
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
    BoxError(Box<dyn Error>),
    Closed,
}

impl Display for SocketError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            SocketError::BindFailed(e) => write!(f, "BindFailed: {}", e),
            SocketError::ConnectionFailed(e) => write!(f, "ConnectionFailed: {}", e),
            SocketError::IoError(e) => write!(f, "IoError: {}", e),
            SocketError::BoxError(e) => write!(f, "BoxError: {}", e),
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

pub trait ToTcpListener {
    type Future: Future<Output = Result<TcpListener, std::io::Error>>;
    fn to_tcp_listener(self) -> Self::Future;
}

impl ToTcpListener for TcpListener {
    type Future = Pin<Box<dyn Future<Output = Result<TcpListener, std::io::Error>>>>;
    fn to_tcp_listener(self) -> Self::Future {
        Box::pin(async move { Ok(self) })
    }
}

impl ToTcpListener for &str {
    type Future = Pin<Box<dyn Future<Output = Result<TcpListener, std::io::Error>> + Send>>;
    fn to_tcp_listener(self) -> Self::Future {
        let this = self.to_string();
        Box::pin(async move { TcpListener::bind(this).await })
    }
}

#[async_trait]
pub trait SocketHandler {
    async fn handle_connection(&mut self, stream: TcpStream) -> Result<(), SocketError>;
}

pub async fn run<L, H>(addr: L, handler: H) -> Result<(), SocketError>
where
    L: ToTcpListener,
    H: SocketHandler + Clone + Send + 'static,
{
    let listener = addr
        .to_tcp_listener()
        .await
        .map_err(SocketError::BindFailed)?;
    println!("SMTP Server listening on {}", listener.local_addr()?);

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
