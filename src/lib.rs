use std::time::Duration;
use uuid::Uuid;

mod network;
mod swim;

pub struct MemberId {
    id: Uuid,
}

pub struct GossipConfig {
    protocol_period: Duration
}