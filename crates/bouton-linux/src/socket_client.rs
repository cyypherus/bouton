use bouton_core::InputEvent;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;

pub struct SocketClient {
    stream: TcpStream,
}

impl SocketClient {
    pub async fn connect(addr: SocketAddr) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream })
    }

    pub async fn send_event(&mut self, event: InputEvent) -> std::io::Result<()> {
        let json = serde_json::to_string(&event)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "encoding failed"))?;
        
        self.stream.write_all(json.as_bytes()).await?;
        self.stream.write_all(b"\n").await?;
        self.stream.flush().await?;
        Ok(())
    }
}
