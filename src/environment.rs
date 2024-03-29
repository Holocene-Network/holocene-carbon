pub use ink_env::{DefaultEnvironment, Environment};

pub type AccountId = <DefaultEnvironment as Environment>::AccountId;
pub type Balance = <DefaultEnvironment as Environment>::Balance;
pub type BlockNumber = <DefaultEnvironment as Environment>::BlockNumber;
pub type Hash = <DefaultEnvironment as Environment>::Hash;
pub type Timestamp = <DefaultEnvironment as Environment>::Timestamp;
