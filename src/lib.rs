#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::vec_init_then_push)]

pub mod custodian;
pub mod environment;
pub mod error;
pub mod retirement;
pub mod token;
pub mod utils;

pub use crate::error::Message as OperationError;
pub use crate::retirement::Report as RetirementReport;
pub use crate::token::{TokenBalanceDetail, TokenEdition};
pub use ink_env::{DefaultEnvironment, Environment};
pub use ink_lang::codegen::initialize_contract;
pub use ink_prelude::string::String;
pub use ink_prelude::vec::Vec as GenericVec;
pub use ink_storage::collections::{HashMap as StorageHashMap, Vec as StorageVec};
pub use ink_storage::traits::{KeyPtr, SpreadAllocate, SpreadLayout};
pub use ink_storage::{Box as StorageBox, Lazy};
pub use scale::{Decode, Encode};

// Type Facades
pub type CarbonUnit = u64;
pub type MintBeneficiaryAccount = environment::AccountId;
pub type RegistryId = String;
pub type RetirementId = u64;
pub type RetirementReports = GenericVec<RetirementReport>;
pub type TokenBalances = GenericVec<TokenBalanceDetail>;
pub type TokenEditions = GenericVec<TokenEdition>;
pub type TokenId = u32;
pub type Year = u16;

#[ink_lang::contract(dynamic_storage_allocator = true)]
pub mod contract {
    use super::*;
    use crate::custodian::{
        AddParams as AddCustodianParams, Collections as Custodians, Detail as CustodianDetail,
    };
    use crate::retirement::{Book as Retirements, Info as RetirementInfo};
    use crate::token::{
        Detail as TokenDetail, MintRequestParams as TokenMintParams, Tracker as Tokens,
    };

    #[ink(event)]
    pub struct TokenMintRequested {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        registry_id: RegistryId,
    }

    #[ink(event)]
    pub struct TokenMintApproved {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        registry_id: RegistryId,
        #[ink(topic)]
        id: TokenId,
    }

    #[ink(event)]
    pub struct TokenMintDenied {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        registry_id: RegistryId,
    }

    #[ink(event)]
    pub struct TokenTransferred {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        editions: TokenEditions,
    }

    #[ink(event)]
    pub struct TokenRetired {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        info: RetirementInfo,
    }

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Wall {
        block_number: Lazy<BlockNumber>,
        governor: Lazy<AccountId>,
        custodians: Lazy<StorageBox<Custodians>>,
        tokens: Lazy<StorageBox<Tokens>>,
        retirements: Lazy<StorageBox<Retirements>>,
    }

    impl Wall {
        #[ink(constructor)]
        pub fn instantiate() -> Self {
            initialize_contract(|contract_context: &mut Self| {
                Lazy::set(&mut contract_context.governor, Self::env().caller());
                Lazy::set(
                    &mut contract_context.block_number,
                    Self::env().block_number(),
                );
                Lazy::set(
                    &mut contract_context.custodians,
                    StorageBox::new(Custodians::default()),
                );
                Lazy::set(
                    &mut contract_context.tokens,
                    StorageBox::new(Tokens::default()),
                );
                Lazy::set(
                    &mut contract_context.retirements,
                    StorageBox::new(Retirements::default()),
                );
            })
        }

        #[ink(message)]
        pub fn any_system_debug_get_governor(&self) -> AccountId {
            *self.governor
        }

        #[ink(message)]
        pub fn any_system_debug_get_blocknumber(&self) -> BlockNumber {
            *self.block_number
        }

        pub fn any_system_debug_get_last_report_id(&self) -> Option<RetirementId> {
            self.retirements.get_last_report_id()
        }

        #[ink(message)]
        pub fn any_system_debug_get_last_minted_id(&self) -> Option<TokenId> {
            self.tokens.get_last_minted_edition_id()
        }

        #[ink(message)]
        pub fn gov_system_terminate(&mut self) -> Result<(), OperationError> {
            if self.env().caller() != *self.governor {
                return Err(OperationError::Unauthorized);
            }

            self.env().terminate_contract(*self.governor);
        }

        #[ink(message)]
        pub fn gov_custodian_account_add(
            &mut self,
            params: AddCustodianParams,
        ) -> Result<(), OperationError> {
            if self.env().caller() != *self.governor {
                return Err(OperationError::Unauthorized);
            }

            self.custodians.add(params)
        }

        #[ink(message)]
        pub fn gov_custodian_account_remove(
            &mut self,
            id: AccountId,
        ) -> Result<(), OperationError> {
            if self.env().caller() != *self.governor {
                return Err(OperationError::Unauthorized);
            }

            self.custodians.remove(id)
        }

        #[ink(message)]
        pub fn any_custodian_account_list(&mut self) -> GenericVec<CustodianDetail> {
            self.custodians.list()
        }

        #[ink(message)]
        pub fn ctd_token_mint_request(
            &mut self,
            params: TokenMintParams,
        ) -> Result<(), OperationError> {
            let minter = self.env().caller();
            let registry_id = params.registry_id.clone();

            if !self.custodians.contains(minter) {
                return Err(OperationError::Unauthorized);
            }

            self.tokens.insert_pending_mint(minter, params)?;
            self.env().emit_event(TokenMintRequested {
                from: minter,
                to: *self.governor,
                registry_id,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn gov_token_mint_approve(
            &mut self,
            registry_id: RegistryId,
        ) -> Result<(), OperationError> {
            if self.env().caller() != *self.governor {
                return Err(OperationError::Unauthorized);
            }

            let (minter_id, target_account_id, token_id, token_amount) =
                self.tokens.approve_pending_mint(&registry_id)?;
            self.env().emit_event(TokenMintApproved {
                from: *self.governor,
                to: minter_id,
                registry_id,
                id: token_id,
            });
            let mut editions = GenericVec::new();
            editions.push(TokenEdition {
                id: token_id,
                amount: token_amount,
            });
            self.env().emit_event(TokenTransferred {
                from: *self.governor,
                to: target_account_id,
                editions,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn gov_token_mint_deny(
            &mut self,
            registry_id: RegistryId,
        ) -> Result<(), OperationError> {
            if self.env().caller() != *self.governor {
                return Err(OperationError::Unauthorized);
            }

            let to = self.tokens.deny_pending_mint(&registry_id)?;
            self.env().emit_event(TokenMintDenied {
                from: *self.governor,
                to,
                registry_id,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn any_token_mint_info_get_last(&mut self) -> Result<TokenDetail, OperationError> {
            self.tokens.get_last_minted_edition_info()
        }

        #[ink(message)]
        pub fn any_token_mint_info_get_by_id(
            &mut self,
            token_id: TokenId,
        ) -> Result<TokenDetail, OperationError> {
            self.tokens.get_minted_edition_by_id(token_id)
        }

        #[ink(message)]
        pub fn any_token_supply_get_total(&mut self) -> CarbonUnit {
            self.tokens.get_total_supply()
        }

        #[ink(message)]
        pub fn any_token_supply_get_by_year(
            &mut self,
            year: Year,
        ) -> Result<CarbonUnit, OperationError> {
            self.tokens.get_supply_by_year(year)
        }

        #[ink(message)]
        pub fn any_token_supply_get_by_id(
            &mut self,
            token_id: TokenId,
        ) -> Result<CarbonUnit, OperationError> {
            self.tokens.get_supply_by_id(token_id)
        }
        #[ink(message)]
        pub fn any_token_retired_supply_get_total(&mut self) -> CarbonUnit {
            self.tokens.get_total_retired()
        }

        #[ink(message)]
        pub fn any_token_retired_supply_get_by_year(
            &mut self,
            year: Year,
        ) -> Result<CarbonUnit, OperationError> {
            self.tokens.get_retired_by_year(year)
        }

        #[ink(message)]
        pub fn any_token_retired_supply_get_by_id(
            &mut self,
            token_id: TokenId,
        ) -> Result<CarbonUnit, OperationError> {
            self.tokens.get_retired_by_id(token_id)
        }

        #[ink(message)]
        pub fn own_token_balance_get_all(&mut self) -> TokenBalances {
            let account_context = self.env().caller();

            self.tokens.get_account_balances(account_context)
        }

        #[ink(message)]
        pub fn own_token_balance_get_by_id(
            &mut self,
            token_id: TokenId,
        ) -> Result<CarbonUnit, OperationError> {
            let account_context = self.env().caller();

            self.tokens
                .get_account_balance_by_id(account_context, token_id)
        }

        #[ink(message)]
        pub fn own_token_balance_get_by_year(
            &mut self,
            year: Year,
        ) -> Result<CarbonUnit, OperationError> {
            let account_context = self.env().caller();

            self.tokens
                .get_account_balance_by_year(account_context, year)
        }

        #[ink(message)]
        pub fn own_token_balance_get_total(&mut self) -> CarbonUnit {
            let account_context = self.env().caller();

            self.tokens.get_account_total_balance(account_context)
        }

        #[ink(message)]
        pub fn own_token_transfer_all(
            &mut self,
            target_account_id: AccountId,
        ) -> Result<(), OperationError> {
            let account_context = self.env().caller();
            let editions = self
                .tokens
                .transfer_token_all(account_context, target_account_id)?;
            self.env().emit_event(TokenTransferred {
                from: account_context,
                to: target_account_id,
                editions,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn own_token_transfer_by_id(
            &mut self,
            target_account_id: AccountId,
            token_id: TokenId,
            token_amount: CarbonUnit,
        ) -> Result<(), OperationError> {
            let account_context = self.env().caller();
            let index = self.tokens.transfer_token_by_id(
                account_context,
                target_account_id,
                token_id,
                token_amount,
            )?;
            let mut editions = GenericVec::new();
            editions.push(index);
            self.env().emit_event(TokenTransferred {
                from: account_context,
                to: target_account_id,
                editions,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn own_token_transfer_by_year(
            &mut self,
            target_account_id: AccountId,
            token_year: Year,
            token_amount: CarbonUnit,
        ) -> Result<(), OperationError> {
            let account_context = self.env().caller();
            let editions = self.tokens.transfer_token_by_year(
                account_context,
                target_account_id,
                token_year,
                token_amount,
            )?;
            self.env().emit_event(TokenTransferred {
                from: account_context,
                to: target_account_id,
                editions,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn own_token_transfer_compounded(
            &mut self,
            target_account_id: AccountId,
            params: TokenEditions,
        ) -> Result<(), OperationError> {
            let account_context = self.env().caller();
            self.tokens
                .transfer_token_compounded(account_context, target_account_id, &params)?;
            self.env().emit_event(TokenTransferred {
                from: account_context,
                to: target_account_id,
                editions: params,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn any_retirement_report_get_by_id(
            &mut self,
            retirement_id: RetirementId,
        ) -> Result<RetirementReport, OperationError> {
            self.retirements.get_report_by_id(retirement_id)
        }

        #[ink(message)]
        pub fn any_retirement_report_get_last(
            &mut self,
        ) -> Result<RetirementReport, OperationError> {
            self.retirements.get_last_report()
        }

        #[ink(message)]
        pub fn own_retirement_report_get_all(&mut self) -> RetirementReports {
            let account_context = self.env().caller();

            self.retirements.get_account_report(account_context)
        }

        #[ink(message)]
        pub fn own_token_retire_by_id(
            &mut self,
            token_id: TokenId,
            retirement_amount: CarbonUnit,
        ) -> Result<RetirementId, OperationError> {
            let account_context = self.env().caller();
            self.tokens
                .retire_token_id(account_context, token_id, retirement_amount)?;
            let token_detail = self.tokens.get_edition_details(token_id)?;
            let token_detail = TokenBalanceDetail {
                balance: retirement_amount,
                detail: token_detail,
            };
            let retirement_info = self
                .retirements
                .insert_new_report(account_context, &token_detail);
            let retirement_id = retirement_info.id;
            self.env().emit_event(TokenRetired {
                from: self.env().account_id(),
                to: account_context,
                info: retirement_info,
            });

            Ok(retirement_id)
        }
    }
}
