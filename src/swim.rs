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
use crate::id::MemberId;
use crate::membership_list::{MembershipList, MemberState, MemberStateType};

use crate::network::{InboundMessage, InMemoryNetworkClient, NetworkClient, OutboundMessage};

struct Config {
    //defines frequency of probing. By increasing the frequency we increase the dissemination speed
    gossip_probe_period: Duration,
}

fn start_swim_member(config: Config,
                     cancellation_token: CancellationToken,
                     protocol_client_mutex: Arc<Mutex<dyn ProtocolClient>>,
                     membership_list: Arc<Mutex<MembershipList>>,
                     network_client: Box<dyn NetworkClient>,
                     receiver: Receiver<InboundMessage>) -> Vec<JoinHandle<()>> {
    let mut result = Vec::new();
    result.push(start_gossip_thread(config, protocol_client_mutex.clone(), membership_list, network_client, cancellation_token.clone()));
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
                       membership_list_mutex: Arc<Mutex<MembershipList>>,
                       network_client: Box<dyn NetworkClient>,
                       cancellation_token: CancellationToken) -> JoinHandle<()> {
    return thread::spawn(move || {
        loop {
            let membership_list = membership_list_mutex.lock().unwrap();
            //TODO - we should pick ALIVE, SUSPECTED and suspected.time_to_transition >=DEAD.config
            let random_members_op = membership_list.get_k_random_members(1,
                                                                         |elem|
                                                                             elem.member_state_type != MemberStateType::ALIVE &&
                                                                             elem.member_state_type != MemberStateType::SUSPECTED);
            drop(membership_list);
            match random_members_op {
                None => {
                    log::debug!("My membership list is empty. I will pause for {:?} and try to probe a member",
                        config.gossip_probe_period);
                    thread::sleep(config.gossip_probe_period);
                }
                Some(target) => {
                    let target_state = &target[0];
                    let new_msg = {
                        let protocol_client = protocol_client_mutex.lock().unwrap();
                        protocol_client.new_message()
                    };
                    let before_probe = time::Instant::now();
                    match network_client.send(OutboundMessage { content: new_msg, target_address: target_state.socket_addr }) {
                        Ok(_) => { log::debug!("Successfully probe {:?}", target_state.socket_addr) }
                        Err(err) => { log::debug!("Wasn't able to probe {:?}, {:?}", target_state.socket_addr, err) }
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

trait ProtocolClient: Sync + Send {
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
    let membership_lists = create_two_membership_lists(server_address_1, server_address_2);
    let join_handlers1 = start_swim_member(Config { gossip_probe_period: Duration::from_secs(1) },
                                           cancellation_token.clone(),
                                           Arc::new(Mutex::new(MockProtocolClient::new(Bytes::from("c")))),
                                           Arc::new(Mutex::new(membership_lists.0)),
                                           Box::new(InMemoryNetworkClient::new(channel_map.clone(), server_address_1)),
                                           server1_receiver);

    let join_handlers2 = start_swim_member(Config { gossip_probe_period: Duration::from_secs(1) },
                                           cancellation_token.clone(),
                                           Arc::new(Mutex::new(MockProtocolClient::new(Bytes::from("b")))),
                                           Arc::new(Mutex::new(membership_lists.1)),
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

fn create_two_membership_lists(server_address_1: SocketAddr, server_address_2: SocketAddr) -> (MembershipList, MembershipList) {
    let member_id_1 = MemberId::generate_random();
    let member_id_2 = MemberId::generate_random();
    let mut membership_list1 = MembershipList::new(member_id_1, server_address_1);
    let mut membership_list2 = MembershipList::new(member_id_2, server_address_2);
    membership_list1.merge(vec![MemberState::alive(member_id_2, server_address_2)]);
    membership_list2.merge(vec![MemberState::alive(member_id_1, server_address_1)]);

    return (membership_list1, membership_list2);
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