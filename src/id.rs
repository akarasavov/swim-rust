use bytes::{Buf, BufMut, Bytes, BytesMut};
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct MemberId(Id);

impl MemberId{
    pub fn generate_random() -> MemberId {
        return MemberId(Id::generate_random());
    }
}

impl MemberId {}

/// 128-bit random ID.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct Id([u8; 16]);

impl Id {
    fn generate_random() -> Id {
        let mut id = [0u8; 16];
        rand::thread_rng().fill(&mut id);
        return Id::from(id);
    }

    fn to_bytes(id: &Id) -> Bytes {
        let mut bytes_mut = BytesMut::with_capacity(16);
        bytes_mut.put_slice(&id.0);
        return bytes_mut.freeze();
    }

    fn from_bytes(buf: &mut Bytes) -> Id {
        let mut id = [0u8; 16];
        buf.copy_to_slice(&mut id);
        return Id::from(id);
    }
}

impl From<[u8; 16]> for Id {
    fn from(b: [u8; 16]) -> Self {
        Id(b)
    }
}

impl From<Id> for u128 {
    fn from(id: Id) -> Self {
        u128::from_le_bytes(id.0)
    }
}

#[test]
fn serialize_deserialize_test() {
    let expected = Id::generate_random();
    //when
    let produced = Id::from_bytes(&mut Id::to_bytes(&expected));
    //then
    assert_eq!(expected, produced);
}