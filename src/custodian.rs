use crate::environment::{AccountId, BlockNumber, DefaultEnvironment, Timestamp};
use crate::{Decode, Encode, GenericVec, OperationError, SpreadLayout, StorageHashMap, String};
use ink_env::{block_number, block_timestamp};
use ink_storage::traits::PackedLayout;

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct AddParams {
    pub id: AccountId,
    pub alias: String,
}

#[derive(Clone, Debug, SpreadLayout, PackedLayout, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct Detail {
    id: AccountId,
    block_number: BlockNumber,
    timestamp: Timestamp,
    alias: String,
}

#[derive(Debug, Default, SpreadLayout)]
pub struct Collections {
    custodians: StorageHashMap<AccountId, Detail>,
}

impl From<AddParams> for Detail {
    fn from(source: AddParams) -> Self {
        Self {
            id: source.id,
            alias: source.alias,
            block_number: block_number::<DefaultEnvironment>(),
            timestamp: block_timestamp::<DefaultEnvironment>(),
        }
    }
}

impl Collections {
    pub fn contains(&self, id: AccountId) -> bool {
        self.custodians.contains_key(&id)
    }

    pub fn add(&mut self, create_params: AddParams) -> Result<(), OperationError> {
        if self.custodians.contains_key(&create_params.id) {
            return Err(OperationError::CustodianAlreadyRegistered);
        }

        self.custodians
            .insert(create_params.id, create_params.into());

        Ok(())
    }

    pub fn remove(&mut self, id: AccountId) -> Result<(), OperationError> {
        if !self.custodians.contains_key(&id) {
            return Err(OperationError::CustodianNotFound);
        }

        self.custodians.take(&id);

        Ok(())
    }

    pub fn list(&self) -> GenericVec<Detail> {
        self.custodians
            .values()
            .cloned()
            .collect::<GenericVec<Detail>>()
    }
}
