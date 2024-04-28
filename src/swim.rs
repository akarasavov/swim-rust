use std::{thread, time};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use bytes_utils::Str;
use rand::prelude::SliceRandom;
use tokio_util::sync::CancellationToken;

use crate::network::{InboundMessage, InMemoryNetworkClient, NetworkClient, OutboundMessage};

struct Config {
    //defines frequency of probing. By increasing the frequency we increase the dissemination speed
    gossip_probe_period: Duration,
}

fn start_swim_member(config: Config,
                     cancellation_token: CancellationToken,
                     protocol_client_mutex: Arc<Mutex<dyn ProtocolClient>>,
                     selection_strategy: Box<dyn SelectionStrategy>,
                     network_client: Box<dyn NetworkClient>,
                     receiver: Receiver<InboundMessage>) -> Vec<JoinHandle<()>> {
    let mut result = Vec::new();
    result.push(start_gossip_thread(config, protocol_client_mutex.clone(), selection_strategy, network_client, cancellation_token.clone()));
    result.push(process_incoming_requests(protocol_client_mutex, receiver, cancellation_token));
    return result;
}

fn process_incoming_requests(protocol_client_mutex: Arc<Mutex<dyn ProtocolClient>>,
                             receiver: Receiver<InboundMessage>,
                             cancellation_token: CancellationToken) -> JoinHandle<()> {
    return thread::spawn(move || {
        loop {
            match receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(incoming_msg) => {
                    log::debug!("Receive incoming msg form {}", incoming_msg.sender_address);
                    let mut protocol_client = protocol_client_mutex.lock().unwrap();
                    protocol_client.merge(incoming_msg.content)
                }
                Err(_) => {}
            }
            if cancellation_token.is_cancelled() {
                log::debug!("Incoming request thread will be stopped");
                break;
            }
        }
    });
}

fn start_gossip_thread(config: Config,
                       protocol_client_mutex: Arc<Mutex<dyn ProtocolClient>>,
                       mut selection_strategy: Box<dyn SelectionStrategy>,
                       network_client: Box<dyn NetworkClient>,
                       cancellation_token: CancellationToken) -> JoinHandle<()> {
    return thread::spawn(move || {
        loop {
            match selection_strategy.next() {
                None => {
                    log::debug!("My membership list is empty. I will pause for {:?} and try to probe a member",
                        config.gossip_probe_period);
                    thread::sleep(config.gossip_probe_period);
                }
                Some(target) => {
                    let new_msg = {
                        let protocol_client = protocol_client_mutex.lock().unwrap();
                        protocol_client.new_message()
                    };
                    let before_probe = time::Instant::now();
                    match network_client.send(OutboundMessage { content: new_msg, target_address: target }) {
                        Ok(_) => { log::debug!("Successfully probe {:?}", target) }
                        Err(err) => { log::debug!("Wasn't able to probe {:?}, {:?}", target, err) }
                    }
                    let after_probe = time::Instant::now();
                    let duration_of_exec = after_probe.duration_since(before_probe);
                    if duration_of_exec < config.gossip_probe_period {
                        let sleep_duration = config.gossip_probe_period - duration_of_exec;
                        log::debug!("Will await for {:?} to make sure that we do probing once per {:?}",
                            sleep_duration,
                            config.gossip_probe_period);
                        thread::sleep(sleep_duration);
                    }
                }
            }
            if cancellation_token.is_cancelled() {
                log::debug!("Probe thread will be stopped");
                break;
            }
        }
    });
}

trait SelectionStrategy: Sync + Send {
    fn next(&mut self) -> Option<SocketAddr>;
}

struct RoundRobinSelectionStrategy {
    members: Vec<SocketAddr>,
    next_index: usize,
}

impl RoundRobinSelectionStrategy {
    fn with_initial_list(initial_addresses: Vec<SocketAddr>) -> RoundRobinSelectionStrategy {
        return RoundRobinSelectionStrategy { members: initial_addresses, next_index: 0 };
    }
}

impl SelectionStrategy for RoundRobinSelectionStrategy {
    fn next(&mut self) -> Option<SocketAddr> {
        if self.members.is_empty() {
            return None;
        }
        return if self.next_index < self.members.len() {
            let result = self.members[self.next_index];
            self.next_index += 1;
            Some(result)
        } else {
            self.members.shuffle(&mut rand::thread_rng());
            self.next_index = 0;
            self.next()
        };
    }
}

trait ProtocolClient : Sync + Send {
    fn new_message(&self) -> Bytes;

    fn merge(&mut self, remote_content: Bytes);
}

#[test]
fn run_two_swim_members_with_in_memory_transport() {
    let _ = env_logger::builder().is_test(true).try_init();
    let server_address_1 = SocketAddr::from_str("127.0.0.1:8081").unwrap();
    let server_address_2 = SocketAddr::from_str("127.0.0.1:8082").unwrap();
    let (server1_sender, server1_receiver) = mpsc::channel();
    let (server2_sender, server2_receiver) = mpsc::channel();
    let mut channel_map = HashMap::new();
    channel_map.insert(server_address_1, server1_sender);
    channel_map.insert(server_address_2, server2_sender);

    let cancellation_token = CancellationToken::new();
    let join_handlers1 = start_swim_member(Config { gossip_probe_period: Duration::from_secs(1) },
                                           cancellation_token.clone(),
                                           Arc::new(Mutex::new(MockProtocolClient::new(Bytes::from("c")))),
                                           Box::new(RoundRobinSelectionStrategy::with_initial_list(vec![server_address_2])),
                                           Box::new(InMemoryNetworkClient::new(channel_map.clone(), server_address_1)),
                                           server1_receiver);

    let join_handlers2 = start_swim_member(Config { gossip_probe_period: Duration::from_secs(1) },
                                           cancellation_token.clone(),
                                           Arc::new(Mutex::new(MockProtocolClient::new(Bytes::from("b")))),
                                           Box::new(RoundRobinSelectionStrategy::with_initial_list(vec![server_address_1])),
                                           Box::new(InMemoryNetworkClient::new(channel_map, server_address_2)),
                                           server2_receiver);

    thread::sleep(Duration::from_secs(3));
    cancellation_token.cancel();
    await_until_join(join_handlers1);
    await_until_join(join_handlers2);
}

fn await_until_join(join_handlers: Vec<JoinHandle<()>>) {
    for join_handler in join_handlers {
        join_handler.join().expect("Fail in await");
    }
}

struct MockProtocolClient {
    content: BytesMut,
}

impl MockProtocolClient {
    fn new(init_state: Bytes) -> MockProtocolClient {
        let mut content = BytesMut::with_capacity(1000);
        content.put(init_state);
        return MockProtocolClient { content };
    }
}

impl ProtocolClient for MockProtocolClient {
    fn new_message(&self) -> Bytes {
        return self.content.clone().freeze();
    }

    fn merge(&mut self, remote_content: Bytes) {
        self.content.put(remote_content);
        log::debug!("{}", Str::try_from(self.content.clone().freeze()).unwrap())
    }
}