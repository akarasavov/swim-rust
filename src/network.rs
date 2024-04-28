use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::Sender;

use bytes::Bytes;

pub trait NetworkClient : Sync + Send {
    fn send(&self, network_message: OutboundMessage) -> anyhow::Result<()>;
}

#[derive(PartialEq)]
#[derive(Debug)]
pub struct OutboundMessage {
    pub content: Bytes,
    pub target_address: SocketAddr,
}

#[derive(PartialEq)]
#[derive(Debug)]
pub struct InboundMessage {
    pub content: Bytes,
    pub sender_address: SocketAddr,
}

pub struct InMemoryNetworkClient {
    channel_map: HashMap<SocketAddr, Sender<InboundMessage>>,
    my_address: SocketAddr,
}

impl NetworkClient for InMemoryNetworkClient {
    fn send(&self, network_message: OutboundMessage) -> anyhow::Result<()> {
        let channel = self.channel_map.get(&network_message.target_address).unwrap();
        channel.send(InboundMessage { content: network_message.content, sender_address: self.my_address })?;
        return anyhow::Ok(());
    }
}

impl InMemoryNetworkClient {
    pub fn new(channel_map: HashMap<SocketAddr, Sender<InboundMessage>>, my_address: SocketAddr) -> InMemoryNetworkClient {
        return InMemoryNetworkClient { channel_map, my_address };
    }
}