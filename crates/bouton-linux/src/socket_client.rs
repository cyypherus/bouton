use bouton_core::InputEvent;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

pub struct SocketClient {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

impl SocketClient {
    pub async fn connect(server_addr: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        Ok(Self { socket, server_addr })
    }

    pub async fn send_event(&mut self, event: InputEvent) -> std::io::Result<()> {
        let bytes = bincode::serialize(&event)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "encoding failed"))?;
        
        self.socket.send_to(&bytes, self.server_addr).await?;
        Ok(())
    }
}
