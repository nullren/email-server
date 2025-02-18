use std::sync::Arc;

pub mod smtp;
pub mod socket;

pub async fn smtp_server(addr: &str) -> Result<(), socket::SocketError> {
    socket::run(
        addr,
        smtp::Server {
            handler: Arc::new(smtp::PrintHandler),
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use crate::smtp_server;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::OnceCell;

    static SERVER_ADDRESS: OnceCell<String> = OnceCell::const_new();

    async fn start_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_address = listener.local_addr().unwrap().to_string();
        drop(listener);

        let addr = server_address.clone();
        tokio::spawn(async move {
            smtp_server(&addr).await.unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        server_address
    }

    async fn get_server_address() -> &'static str {
        SERVER_ADDRESS.get_or_init(start_server).await
    }

    #[tokio::test]
    async fn test_smtp_server_initial_response() {
        let server_address = get_server_address().await;

        // Connect to the SMTP server
        let mut stream = TcpStream::connect(server_address).await.unwrap();

        // Read the server's initial response
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("220"));
    }

    #[tokio::test]
    async fn test_smtp_server_helo_command() {
        let server_address = get_server_address().await;

        // Connect to the SMTP server
        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        // Send HELO command
        stream.write_all(b"HELO example.com\r\n").await.unwrap();
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("250"));
    }

    #[tokio::test]
    async fn test_smtp_server_quit_command() {
        let server_address = get_server_address().await;

        // Connect to the SMTP server
        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        // Send QUIT command
        stream.write_all(b"QUIT\r\n").await.unwrap();
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        println!("QUIT response: {}", response);
        assert!(response.starts_with("221"));
    }

    #[tokio::test]
    async fn test_full_message() {
        let server_address = get_server_address().await;

        // Connect to the SMTP server
        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        // Send HELO command
        stream.write_all(b"HELO example.com\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("250"));

        // Send MAIL FROM command
        stream
            .write_all(b"MAIL FROM: Alice <Alice@example.com>\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("250"));

        // Send RCPT TO command
        stream
            .write_all(b"RCPT TO: Bob <bob@example.com>\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("250"));

        // Send DATA command
        stream.write_all(b"DATA\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("354"));

        // Send message body
        stream
            .write_all(b"Subject: Test\r\n\r\nHello, world!\r\n.\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("250"));

        // Send QUIT command
        stream.write_all(b"QUIT\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("221"));
    }
}
