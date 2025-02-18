pub mod smtp;
pub mod socket;

pub async fn smtp_server(addr: &str) -> Result<(), socket::SocketError> {
    socket::run(addr, smtp::Server {}).await
}

#[cfg(test)]
mod tests {
    use crate::smtp_server;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn test_smtp_server() {
        // Bind to port 0 to get a random available port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_address = listener.local_addr().unwrap().to_string();
        drop(listener);

        println!("Server address: {}", server_address);

        let addr = server_address.clone();
        tokio::spawn(async move {
            smtp_server(&addr).await.unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Connect to the SMTP server
        let mut stream = TcpStream::connect(server_address).await.unwrap();

        println!("Connected to server");

        // Read the server's initial response
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("220"));

        println!("Received initial response: {}", response);

        // Send HELO command
        stream.write_all(b"HELO example.com\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("250"));

        println!("Received response to HELO: {}", response);

        // Send QUIT command
        stream.write_all(b"QUIT\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("221"));
    }
}
