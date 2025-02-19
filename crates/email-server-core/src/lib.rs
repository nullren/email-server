use crate::socket::ToTcpListener;
use std::sync::Arc;

pub mod message;
pub mod smtp;
pub mod socket;

pub async fn smtp_server<L: ToTcpListener>(addr: L) -> Result<(), socket::SocketError> {
    socket::run(
        addr,
        smtp::Server {
            handler: Arc::new(message::PrintHandler),
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use crate::smtp_server;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    async fn start_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        let local_addr = listener.local_addr().unwrap();
        let addr_str = local_addr.to_string();

        tokio::spawn(async move {
            if let Err(e) = smtp_server(listener).await {
                eprintln!("SMTP Server Error: {}", e);
            }
        });

        // Ensure the server is actually accepting connections
        for _ in 0..10 {
            match TcpStream::connect(&addr_str).await {
                Ok(_) => return addr_str, // If we connect successfully, return address
                Err(_) => tokio::time::sleep(tokio::time::Duration::from_millis(50)).await,
            }
        }

        panic!("Server failed to start");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_smtp_server_initial_response() {
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("220"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_smtp_server_helo_command() {
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        stream.write_all(b"HELO example.com\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("250"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_smtp_server_quit_command() {
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        stream.write_all(b"QUIT\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("221"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_full_message() {
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        stream.write_all(b"HELO example.com\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("250"));

        stream
            .write_all(b"MAIL FROM: Alice <Alice@example.com>\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("250"));

        stream
            .write_all(b"RCPT TO: Bob <bob@example.com>\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("250"));

        stream.write_all(b"DATA\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("354"));

        stream
            .write_all(b"Subject: Test\r\n\r\nHello, world!\r\n.\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("250"));

        stream.write_all(b"QUIT\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("221"));
    }
}
