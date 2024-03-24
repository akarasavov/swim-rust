use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Receiver, Sender};

use bytes::{Bytes, BytesMut};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

struct UdpServer {
    address: SocketAddr,
    cancellation_token: CancellationToken,
}

impl UdpServer {
    fn new(address: SocketAddr) -> UdpServer {
        return UdpServer { address, cancellation_token: CancellationToken::new() };
    }

    async fn start(self) -> anyhow::Result<Receiver<UdpMessage>> {
        let server_socket = Arc::new(tokio::spawn(async move {
            return UdpSocket::bind(self.address).await.unwrap();
        }).await.unwrap());
        let (sender, receiver) = mpsc::channel();
        tokio::spawn(async move {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    Ok(())
                }
                result = Self::process_incoming_request(server_socket, sender) => {
                    result
                }
            }
        });
        return Ok(receiver);
    }

    async fn process_incoming_request(socket: Arc<UdpSocket>, sender: Sender<UdpMessage>) -> anyhow::Result<()> {
        println!("Start worker");
        loop {
            let mut bytes_mut: BytesMut = BytesMut::with_capacity(1024);
            let (len, socket_address) = socket.recv_buf_from(&mut bytes_mut).await?;
            println!("Receive msg with len {} from {}", len, socket_address);
            let message = UdpMessage { bytes: bytes_mut.split_to(len).freeze(), socket_addr: socket_address };
            sender.send(message).unwrap();
        }
    }

    fn stop(self) {
        self.cancellation_token.cancel();
    }
}

#[derive(PartialEq)]
#[derive(Debug)]
struct UdpMessage {
    bytes: Bytes,
    socket_addr: SocketAddr,
}

struct UdpClient {
    socket: UdpSocket,
}

impl UdpClient {
    async fn new(client_address: SocketAddr) -> UdpClient {
        let socket = UdpSocket::bind(client_address).await.unwrap();
        return UdpClient { socket };
    }

    async fn send_to(self, target: SocketAddr, content: Bytes) -> anyhow::Result<()> {
        let mut content_ref = content;
        loop {
            let size = self.socket.send_to(content_ref.as_ref(), target).await?;
            if size == content_ref.len() {
                println!("Successfully send msg to {}", target);
                break;
            }
            content_ref = content_ref.slice(size + 1..content_ref.len());
        }
        return Ok(());
    }
}

#[test]
fn should_received_and_send_messages() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("example_thread")
        .enable_all()
        .build()
        .expect("Failed to create example_thread runtime");
    let server_address = SocketAddr::from_str("127.0.0.1:8080").unwrap();
    let server = UdpServer::new(server_address);
    let receiver = runtime.block_on(async { server.start().await.unwrap() });

    let client_address = SocketAddr::from_str("127.0.0.1:8081").unwrap();
    runtime.block_on(async move {
        let client = UdpClient::new(client_address).await;
        client.send_to(server_address, Bytes::from("test")).await.unwrap();
    });

    let result = receiver.recv().unwrap();
    let expected = UdpMessage { bytes: Bytes::from("test"), socket_addr: client_address };
    assert_eq!(expected, result)
}