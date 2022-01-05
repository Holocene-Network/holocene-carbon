use crate::environment::{AccountId, BlockNumber, DefaultEnvironment, Timestamp};
use crate::utils::get_blackhole_address;
use crate::{
    CarbonUnit, Decode, Encode, GenericVec, MintBeneficiaryAccount, OperationError, RegistryId,
    SpreadLayout, StorageBox, StorageHashMap, StorageVec, TokenEditions, TokenId, Year,
};
use ink_env::{block_number, block_timestamp};
use ink_storage::traits::PackedLayout;

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct MintRequestParams {
    pub registry_id: RegistryId,
    pub verified_carbon_unit: CarbonUnit,
    pub issuance_year: Year,
    pub beneficiary: MintBeneficiaryAccount,
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct TokenEdition {
    pub id: TokenId,
    pub amount: CarbonUnit,
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct TokenBalanceDetail {
    pub balance: CarbonUnit,
    pub detail: Detail,
}

#[derive(Clone, Debug, SpreadLayout, PackedLayout, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct Detail {
    pub id: TokenId,
    pub block_number: BlockNumber,
    pub timestamp: Timestamp,
    pub minter: AccountId,
    pub supply: CarbonUnit,
    pub retired: CarbonUnit,
    pub year: Year,
    pub registry_id: RegistryId,
}

#[derive(Debug, Default, SpreadLayout)]
pub struct Tracker {
    next_token_id: TokenId,
    last_minted_token_id: Option<TokenId>,
    minted_editions: StorageHashMap<TokenId, Detail>,
    pending_mint_editions: StorageHashMap<RegistryId, (Detail, MintBeneficiaryAccount)>,
    balances: StorageHashMap<AccountId, StorageBox<StorageHashMap<TokenId, CarbonUnit>>>,
    year_mapping: StorageHashMap<Year, StorageBox<StorageVec<TokenId>>>,
}

impl Tracker {
    pub fn take_next_token_id(&mut self) -> TokenId {
        let next_token_id = self.next_token_id;
        self.next_token_id += 1;

        next_token_id
    }

    pub fn insert_pending_mint(
        &mut self,
        minter: AccountId,
        params: MintRequestParams,
    ) -> Result<(), OperationError> {
        if self.pending_mint_editions.contains_key(&params.registry_id) {
            return Err(OperationError::TokenMintRequestAlreadyPending);
        }

        let detail = Detail {
            id: self.take_next_token_id(),
            registry_id: params.registry_id.clone(),
            supply: params.verified_carbon_unit,
            retired: 0,
            year: params.issuance_year,
            minter,
            block_number: block_number::<DefaultEnvironment>(),
            timestamp: block_timestamp::<DefaultEnvironment>(),
        };
        self.pending_mint_editions
            .insert(params.registry_id, (detail, params.beneficiary));

        Ok(())
    }

    pub fn deny_pending_mint(
        &mut self,
        registry_id: &RegistryId,
    ) -> Result<AccountId, OperationError> {
        match self.pending_mint_editions.take(registry_id) {
            None => Err(OperationError::TokenMintRequestNotFound),
            Some((detail, _)) => Ok(detail.minter),
        }
    }

    pub fn approve_pending_mint(
        &mut self,
        registry_id: &RegistryId,
    ) -> Result<(AccountId, MintBeneficiaryAccount, TokenId, CarbonUnit), OperationError> {
        match self.pending_mint_editions.take(registry_id) {
            None => Err(OperationError::TokenMintRequestNotFound),
            Some((detail, target_account_id)) => {
                let minter = detail.minter;
                let token_id = detail.id;
                let token_year = detail.year;
                let token_supply = detail.supply;
                self.minted_editions.insert(detail.id, detail.clone());

                if !self.balances.contains_key(&target_account_id) {
                    self.balances
                        .insert(target_account_id, StorageBox::new(StorageHashMap::new()));
                }

                if !self.year_mapping.contains_key(&token_year) {
                    self.year_mapping
                        .insert(token_year, StorageBox::new(StorageVec::new()));
                }

                let target_account_balance = self.balances.get_mut(&target_account_id).unwrap();
                target_account_balance.insert(detail.id, detail.supply);
                let year_mapping = self.year_mapping.get_mut(&token_year).unwrap();
                year_mapping.push(token_id);
                self.last_minted_token_id = Some(token_id);

                Ok((minter, target_account_id, token_id, token_supply))
            }
        }
    }

    pub fn get_edition_details(&self, id: TokenId) -> Result<Detail, OperationError> {
        match self.minted_editions.get(&id) {
            None => Err(OperationError::TokenNotFound),
            Some(detail) => Ok(detail.clone()),
        }
    }

    pub fn get_last_minted_edition_id(&self) -> Option<TokenId> {
        self.last_minted_token_id
    }

    pub fn get_minted_edition_by_id(&self, token_id: TokenId) -> Result<Detail, OperationError> {
        match self.minted_editions.get(&token_id) {
            None => Err(OperationError::TokenNotFound),
            Some(detail) => Ok(detail.clone()),
        }
    }

    pub fn get_last_minted_edition_info(&self) -> Result<Detail, OperationError> {
        if let Some(last_minted_id) = self.last_minted_token_id {
            self.get_minted_edition_by_id(last_minted_id)
        } else {
            Err(OperationError::TokenNotFound)
        }
    }

    pub fn get_total_supply(&self) -> CarbonUnit {
        let mut total_supply = 0;

        for detail in self.minted_editions.values() {
            total_supply += detail.supply;
        }

        total_supply
    }

    pub fn get_supply_by_id(&self, token_id: TokenId) -> Result<CarbonUnit, OperationError> {
        match self.minted_editions.get(&token_id) {
            None => Err(OperationError::TokenNotFound),
            Some(detail) => Ok(detail.supply),
        }
    }

    pub fn get_supply_by_year(&self, year: Year) -> Result<CarbonUnit, OperationError> {
        match self.year_mapping.get(&year) {
            None => Err(OperationError::TokenNotFound),
            Some(token_indices) => {
                let mut year_supply = 0;
                let token_indices = token_indices.into_iter();

                for token_id in token_indices {
                    if let Some(detail) = self.minted_editions.get(token_id) {
                        year_supply += detail.supply;
                    }
                }

                Ok(year_supply)
            }
        }
    }

    pub fn get_total_retired(&self) -> CarbonUnit {
        let mut total_retired = 0;

        for detail in self.minted_editions.values() {
            total_retired += detail.retired;
        }

        total_retired
    }

    pub fn get_retired_by_id(&self, token_id: TokenId) -> Result<CarbonUnit, OperationError> {
        match self.minted_editions.get(&token_id) {
            None => Err(OperationError::TokenNotFound),
            Some(detail) => Ok(detail.retired),
        }
    }

    pub fn get_retired_by_year(&self, year: Year) -> Result<CarbonUnit, OperationError> {
        match self.year_mapping.get(&year) {
            None => Err(OperationError::TokenNotFound),
            Some(token_indices) => {
                let mut year_retired_supply = 0;
                let token_indices = token_indices.into_iter();

                for token_id in token_indices {
                    if let Some(detail) = self.minted_editions.get(token_id) {
                        year_retired_supply += detail.retired;
                    }
                }

                Ok(year_retired_supply)
            }
        }
    }

    pub fn get_account_balances(&self, account_id: AccountId) -> GenericVec<TokenBalanceDetail> {
        match self.balances.get(&account_id) {
            None => GenericVec::new(),
            Some(account_balances) => {
                let account_balances = account_balances.iter();
                let mut token_details = GenericVec::new();

                for (token_id, token_balance) in account_balances {
                    token_details.push(TokenBalanceDetail {
                        detail: self.minted_editions.get(token_id).unwrap().clone(),
                        balance: *token_balance,
                    });
                }

                token_details
            }
        }
    }

    pub fn get_account_total_balance(&self, account_id: AccountId) -> CarbonUnit {
        match self.balances.get(&account_id) {
            None => 0,
            Some(account_balances) => {
                let mut total_balance = 0;
                let account_balances = account_balances.values();

                for token_balance in account_balances {
                    total_balance += *token_balance;
                }

                total_balance
            }
        }
    }

    pub fn get_account_balance_by_id(
        &self,
        account_id: AccountId,
        token_id: TokenId,
    ) -> Result<CarbonUnit, OperationError> {
        if !self.minted_editions.contains_key(&token_id) {
            return Err(OperationError::TokenNotFound);
        }

        match self.balances.get(&account_id) {
            None => Ok(0),
            Some(token_balances) => match token_balances.get(&token_id) {
                None => Ok(0),
                Some(token_balance) => Ok(*token_balance),
            },
        }
    }

    pub fn get_account_balance_by_year(
        &self,
        account_id: AccountId,
        year: Year,
    ) -> Result<CarbonUnit, OperationError> {
        if !self.year_mapping.contains_key(&year) {
            return Err(OperationError::TokenNotFound);
        }

        let year_tokens = self.year_mapping.get(&year).unwrap().into_iter();

        match self.balances.get(&account_id) {
            None => Ok(0),
            Some(token_balances) => {
                if token_balances.is_empty() {
                    return Ok(0);
                }

                let mut total_year_token = 0;

                for token_id in year_tokens {
                    if let Some(token_balance) = token_balances.get(token_id) {
                        total_year_token += *token_balance;
                    }
                }

                Ok(total_year_token)
            }
        }
    }

    pub fn transfer_token_all(
        &mut self,
        account_id: AccountId,
        target_account_id: AccountId,
    ) -> Result<TokenEditions, OperationError> {
        if self.get_account_total_balance(account_id) == 0 {
            return Err(OperationError::CannotTransferZeroCarbonUnit);
        }

        let mut transfer_details = GenericVec::new();

        if !self.balances.contains_key(&target_account_id) {
            self.balances
                .insert(target_account_id, StorageBox::new(StorageHashMap::new()));
        }

        if !self.balances.contains_key(&account_id) {
            return Err(OperationError::InsufficientCarbonUnit);
        }

        let context_account_balances = self.balances.get_mut(&account_id).unwrap().as_mut();

        for (token_id, token_amount) in context_account_balances.into_iter() {
            transfer_details.push(TokenEdition {
                id: *token_id,
                amount: *token_amount,
            });
        }

        *context_account_balances = StorageHashMap::new();
        let target_account_balances = self.balances.get_mut(&target_account_id).unwrap().as_mut();

        for token_edition in &transfer_details {
            match target_account_balances.get_mut(&token_edition.id) {
                None => {
                    target_account_balances.insert(token_edition.id, token_edition.amount);
                }
                Some(target_account_balance) => *target_account_balance += token_edition.amount,
            }
        }

        Ok(transfer_details)
    }

    pub fn transfer_token_by_id(
        &mut self,
        account_id: AccountId,
        target_account_id: AccountId,
        token_id: TokenId,
        token_amount: CarbonUnit,
    ) -> Result<TokenEdition, OperationError> {
        if token_amount == 0 {
            return Err(OperationError::CannotTransferZeroCarbonUnit);
        }

        if !self.balances.contains_key(&target_account_id) {
            self.balances
                .insert(target_account_id, StorageBox::new(StorageHashMap::new()));
        }

        if !self.balances.contains_key(&account_id) {
            return Err(OperationError::InsufficientCarbonUnit);
        }

        if self.get_account_balance_by_id(account_id, token_id)? < token_amount {
            return Err(OperationError::InsufficientCarbonUnit);
        }

        let context_account_balances = self.balances.get_mut(&account_id).unwrap().as_mut();

        if !context_account_balances.contains_key(&token_id) {
            return Err(OperationError::InsufficientCarbonUnit);
        }

        let context_account_balance = context_account_balances.get_mut(&token_id).unwrap();

        if *context_account_balance < token_amount {
            return Err(OperationError::InsufficientCarbonUnit);
        }

        *context_account_balance -= token_amount;

        if *context_account_balance == 0 {
            context_account_balances.take(&token_id);
        }

        let target_account_balances = self.balances.get_mut(&target_account_id).unwrap().as_mut();

        if let Some(target_account_balance) = target_account_balances.get_mut(&token_id) {
            *target_account_balance += token_amount;
        } else {
            target_account_balances.insert(token_id, token_amount);
        }

        Ok(TokenEdition {
            id: token_id,
            amount: token_amount,
        })
    }

    pub fn transfer_token_by_year(
        &mut self,
        account_id: AccountId,
        target_account_id: AccountId,
        token_year: Year,
        token_amount: CarbonUnit,
    ) -> Result<TokenEditions, OperationError> {
        if token_amount == 0 {
            return Err(OperationError::CannotTransferZeroCarbonUnit);
        }

        let mut transfer_details = GenericVec::new();

        if !self.balances.contains_key(&target_account_id) {
            self.balances
                .insert(target_account_id, StorageBox::new(StorageHashMap::new()));
        }

        if !self.balances.contains_key(&account_id) {
            return Err(OperationError::InsufficientCarbonUnit);
        }

        if !self.year_mapping.contains_key(&token_year) {
            return Err(OperationError::TokenNotFound);
        }

        if self.get_account_balance_by_year(account_id, token_year)? < token_amount {
            return Err(OperationError::InsufficientCarbonUnit);
        }

        let year_tokens = self.year_mapping.get(&token_year).unwrap().into_iter();
        let context_account_balances = self.balances.get_mut(&account_id).unwrap().as_mut();
        let mut pending_removal = GenericVec::new();
        let mut remaining_amount_to_transfer = token_amount;

        for token_id in year_tokens {
            if remaining_amount_to_transfer == 0 {
                break;
            }

            if let Some(context_account_year_balance) = context_account_balances.get_mut(token_id) {
                let transferred_balance_by_id;

                if *context_account_year_balance < remaining_amount_to_transfer {
                    transferred_balance_by_id = *context_account_year_balance;
                    *context_account_year_balance = 0;
                    remaining_amount_to_transfer -= *context_account_year_balance;
                } else {
                    transferred_balance_by_id = remaining_amount_to_transfer;
                    *context_account_year_balance -= remaining_amount_to_transfer;
                    remaining_amount_to_transfer = 0;
                }

                if *context_account_year_balance == 0 {
                    pending_removal.push(*token_id);
                }

                transfer_details.push(TokenEdition {
                    id: *token_id,
                    amount: transferred_balance_by_id,
                });
            }
        }

        for token_id in pending_removal.into_iter() {
            context_account_balances.take(&token_id);
        }

        let target_account_balances = self.balances.get_mut(&target_account_id).unwrap().as_mut();

        for token_edition in &transfer_details {
            if let Some(target_account_balance) = target_account_balances.get_mut(&token_edition.id)
            {
                *target_account_balance += token_edition.amount;
            } else {
                target_account_balances.insert(token_edition.id, token_edition.amount);
            }
        }

        Ok(transfer_details)
    }

    pub fn transfer_token_compounded(
        &mut self,
        account_id: AccountId,
        target_account_id: AccountId,
        params: &TokenEditions,
    ) -> Result<(), OperationError> {
        if !self.balances.contains_key(&account_id) {
            return Err(OperationError::InsufficientCarbonUnit);
        }

        let context_account_balances = self.balances.get(&account_id).unwrap();

        for token_edition in params {
            if !self.minted_editions.contains_key(&token_edition.id) {
                return Err(OperationError::TokenNotFound);
            }

            if token_edition.amount == 0 {
                return Err(OperationError::CannotTransferZeroCarbonUnit);
            }

            if !context_account_balances.contains_key(&token_edition.id) {
                return Err(OperationError::InsufficientCarbonUnit);
            }

            if !*context_account_balances.get(&token_edition.id).unwrap() < token_edition.amount {
                return Err(OperationError::InsufficientCarbonUnit);
            }
        }

        for token_edition in params {
            let _ = self.transfer_token_by_id(
                account_id,
                target_account_id,
                token_edition.id,
                token_edition.amount,
            )?;
        }

        Ok(())
    }

    pub fn retire_token_id(
        &mut self,
        account_id: AccountId,
        token_id: TokenId,
        retirement_amount: CarbonUnit,
    ) -> Result<(), OperationError> {
        self.transfer_token_by_id(
            account_id,
            get_blackhole_address(),
            token_id,
            retirement_amount,
        )?;
        let edition_detail = self.minted_editions.get_mut(&token_id).unwrap();

        if edition_detail.supply < retirement_amount {
            return Err(OperationError::BlockchainCorrupted);
        }

        edition_detail.supply -= retirement_amount;
        edition_detail.retired += retirement_amount;

        Ok(())
    }
}
