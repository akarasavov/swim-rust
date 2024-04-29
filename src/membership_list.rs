use std::collections::HashMap;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use rand::{Rng, thread_rng};

use crate::id::MemberId;
use crate::membership_list::MemberStateType::ALIVE;

#[derive(Debug)]
pub struct MembershipList {
    my_id: MemberId,
    members_states: HashMap<MemberId, Arc<MemberState>>,
    states: Vec<Arc<MemberState>>, //all states expect mine
}

impl MembershipList {
    pub fn new(my_id: MemberId, my_socket_addr: SocketAddr) -> MembershipList {
        let mut members_states = HashMap::new();
        members_states.insert(my_id, Arc::new(MemberState::alive(my_id, my_socket_addr)));
        return MembershipList { my_id, members_states, states: Vec::new() };
    }

    pub fn merge(&mut self, remote_state: Vec<MemberState>) {
        for remote_state in remote_state.into_iter() {
            match self.members_states.get(&remote_state.member_id) {
                None => {
                    let new_state = Arc::new(remote_state);
                    self.members_states.insert(remote_state.member_id, new_state.clone());
                    self.states.push(new_state);
                }
                Some(local_state) => {

                }
            }
        }
    }

    pub fn get_k_random_members<P>(&self, k: usize, exclude: P) -> Option<Vec<Arc<MemberState>>>
        where P: FnOnce(Arc<MemberState>) -> bool + Copy,
    {
        if k > self.states.len() {
            return None;
        }
        let mut rng = thread_rng();
        let mut result = Vec::new();
        for _ in 0..k * 3 {
            let index = rng.gen_range(0..self.states.len());
            let picked_state = self.states[index].clone();
            if !exclude(picked_state.clone()) {
                result.push(picked_state);
            }

            if result.len() == k {
                return Some(result);
            }
        }
        return None;
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct MemberState {
    member_id: MemberId,
    pub socket_addr: SocketAddr,
    pub member_state_type: MemberStateType,
    incarnation_number: u32,
    state_changed: Instant,
}

impl MemberState {
    pub fn alive(member_id: MemberId, socket_addr: SocketAddr) -> MemberState {
        return MemberState {
            member_id,
            socket_addr,
            member_state_type: ALIVE,
            incarnation_number: 0,
            state_changed: Instant::now(),
        };
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum MemberStateType {
    ALIVE,
    SUSPECTED,
    DEAD,
    LEFT,
}

impl MemberStateType {
    fn get_code(&self) -> u8 {
        return match self {
            MemberStateType::ALIVE => { 0 }
            MemberStateType::SUSPECTED => { 1 }
            MemberStateType::DEAD => { 2 }
            MemberStateType::LEFT => { 3 }
        };
    }

    fn from_code(code: u8) -> MemberStateType {
        return match code {
            0 => { return MemberStateType::ALIVE; }
            1 => { return MemberStateType::SUSPECTED; }
            2 => { return MemberStateType::DEAD; }
            3 => { return MemberStateType::LEFT; }
            _ => { panic!("{}", format!("Not supported code {}", code)); }
        };
    }
}

#[test]
fn pick_k_random_members_should_fail() {
    let membership_list = random_membership_list();
    assert_eq!(None, membership_list.get_k_random_members(1, |elem| false));
}

#[test]
fn should_successfully_pick_k_members() {
    let mut membership_list = random_membership_list();
    let other_state = MemberState::alive(MemberId::generate_random(), "127.0.0.1:8081".parse().unwrap());
    let vec = Vec::from([other_state.clone()]);
    membership_list.merge(vec);
    {
        //when
        let random_k = membership_list.get_k_random_members(1, |elem| elem.member_state_type != ALIVE)
            .unwrap();
        //then
        assert_eq!(Arc::new(other_state), random_k[0]);
    }
}

fn random_membership_list() -> MembershipList {
    return MembershipList::new(MemberId::generate_random(), "127.0.0.1:8080".parse().unwrap());
}