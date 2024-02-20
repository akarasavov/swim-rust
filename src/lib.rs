use std::collections::HashSet;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::Relaxed;
use crate::State::{INITIAL, STARTED, STOPPED};

mod swim;

trait Lifecycle {
    fn start(&mut self) -> Result<String, String>;

    fn stop(&mut self) -> Result<String, String>;
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Hash)]
enum State {
    INITIAL,
    STARTED,
    STOPPED,
}

pub struct SafeLifecycle {
    state: State,
}

impl SafeLifecycle {
    fn validate_state(curr_state: &State, valid_states: HashSet<State>) -> bool {
        if valid_states.contains(curr_state) {
            return true;
        }
        return false;
    }

    fn is_in_state(&self, expected_state: State) -> bool {
        return self.state == expected_state;
    }
}

impl Lifecycle for SafeLifecycle {
    fn start(&mut self) -> Result<String, String> {
        if !SafeLifecycle::validate_state(&self.state, HashSet::from([STOPPED, INITIAL])) {
            return Err(String::from("State is already started"));
        }
        self.state = STARTED;
        return Ok(String::from("Started"));
    }

    fn stop(&mut self) -> Result<String, String> {
        if !SafeLifecycle::validate_state(&self.state, HashSet::from([STARTED])) {
            return Err(format!("state is {:?}", self.state));
        }
        self.state = STOPPED;
        return Ok("Started".to_string());
    }
}