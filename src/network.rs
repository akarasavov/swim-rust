use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, mpsc};
use std::sync::mpsc::Sender;

use bytes::{Bytes, BytesMut};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

async fn start_udp_server(cancellation_token: CancellationToken,
                          socket: Arc<UdpSocket>,
                          sender: Sender<UdpMessage>) -> anyhow::Result<()> {
    let result = tokio::spawn(async move {
        tokio::select! {
                _ = cancellation_token.cancelled() => {
                    Ok(())
                }
                result = process_incoming_request(socket, sender) => {
                    result
                }
            }
    }).await?;
    return result;
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
#[derive(PartialEq)]
#[derive(Debug)]
struct UdpMessage {
    bytes: Bytes,
    socket_addr: SocketAddr,
}

async fn send_message(socket: Arc<UdpSocket>, bytes: Bytes, socket_addr: SocketAddr) -> anyhow::Result<()> {
    let i = socket.send_to(bytes.as_ref(), socket_addr).await?;
    println!("Send {} from {}", i, bytes.len());
    return Ok(());
}

#[test]
fn should_received_and_send_messages() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("example_thread")
        .enable_all()
        .build()
        .expect("Failed to create example_thread runtime");
    let server_address = "0.0.0.0:8080";
    let server_socket = Arc::new(runtime.block_on(async {
        return UdpSocket::bind(server_address).await.unwrap();
    }));
    let (sender, receiver) = mpsc::channel();
    let cancellation_token = CancellationToken::new();
    runtime.spawn(async { start_udp_server(cancellation_token, server_socket, sender).await.unwrap() });

    let client_address = "127.0.0.1:8081";
    runtime.block_on(async {
        let client_socket = UdpSocket::bind(client_address).await.unwrap();
        client_socket.send_to(Bytes::from("test").as_ref(), server_address).await.unwrap();
    });

    let result = receiver.recv().unwrap();
    let expected = UdpMessage { bytes: Bytes::from("test"), socket_addr: SocketAddr::from_str("127.0.0.1:8081").unwrap() };
    assert_eq!(expected, result)
}