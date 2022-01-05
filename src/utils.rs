use crate::environment::AccountId;

pub fn get_blackhole_address() -> AccountId {
    AccountId::from([0x00; 32])
}
