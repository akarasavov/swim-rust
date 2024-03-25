use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use bytes::Bytes;
use rand::seq::SliceRandom;
use tokio::time::sleep;

use crate::MemberId;
use crate::network::UdpClient;

trait MemberSelectionStrategy {
    fn next(&mut self) -> Option<SocketAddr>;
}

struct RoundRobinSelectionStrategy {
    members: Vec<SocketAddr>,
    next_index: usize,
}

impl RoundRobinSelectionStrategy {
    fn with_initial_list(vec: Vec<SocketAddr>) -> RoundRobinSelectionStrategy {
        return RoundRobinSelectionStrategy { members: vec, next_index: 0 };
    }

    pub fn new() -> RoundRobinSelectionStrategy {
        return RoundRobinSelectionStrategy { members: vec![], next_index: 0 };
    }
}

impl MemberSelectionStrategy for RoundRobinSelectionStrategy {
    fn next(&mut self) -> Option<SocketAddr> {
        if self.members.is_empty() {
            return None;
        }
        if self.next_index < self.members.len() {
            let result = self.members[self.next_index];
            self.next_index += 1;
            return Some(result);
        } else {
            self.members.shuffle(&mut rand::thread_rng());
            self.next_index = 0;
            return self.next();
        }
    }
}

struct SwimMember<'a> {
    id: MemberId,
    selection_strategy: &'a mut dyn MemberSelectionStrategy,
    network_client: UdpClient,
}

impl<'a> SwimMember<'a> {
    // async fn start(self) -> anyhow::Result<()> {
    //     // tokio::spawn(async {
    //     //     Self::start_gossip_algorithm()
    //     // })
    //     // return Ok(());
    // }

    async fn start_gossip_algorithm(self) -> anyhow::Result<()> {
        loop {
            match self.selection_strategy.next() {
                None => {
                    println!("There are no members expect me");
                    sleep(Duration::from_secs(1)).await;
                }
                Some(target) => {}
            }
        }
    }

    async fn execute_probing_algorithm(self, target: SocketAddr) {
        self.network_client.send_to(target, Bytes::from("a")).await;
    }
}


#[test]
fn should_get_next_elem() {
    let first = SocketAddr::from_str("127.0.0.1:8081").unwrap();
    let second = SocketAddr::from_str("127.0.0.1:8082").unwrap();
    let mut strategy = RoundRobinSelectionStrategy::with_initial_list(vec![first, second]);

    //when
    assert_eq!(Some(first), strategy.next());
    assert_eq!(Some(second), strategy.next());
}

#[test]
fn should_return_empty_if_list_is_empty() {
    let mut strategy = RoundRobinSelectionStrategy::with_initial_list(vec![]);

    assert_eq!(None, strategy.next());
}