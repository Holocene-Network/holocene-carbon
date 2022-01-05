use crate::environment::{AccountId, BlockNumber, DefaultEnvironment, Timestamp};
use crate::{
    CarbonUnit, Decode, Encode, GenericVec, OperationError, RegistryId, RetirementId,
    RetirementReports, SpreadLayout, StorageBox, StorageHashMap, StorageVec, TokenBalanceDetail,
    TokenId,
};
use ink_env::{block_number, block_timestamp};
use ink_storage::traits::PackedLayout;

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct Info {
    pub id: RetirementId,
    pub amount: CarbonUnit,
}

#[derive(Clone, Debug, SpreadLayout, PackedLayout, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct Report {
    id: RetirementId,
    block_number: BlockNumber,
    timestamp: Timestamp,
    beneficiary: AccountId,
    token_id: TokenId,
    amount: CarbonUnit,
    registry_id: RegistryId,
}

#[derive(Debug, Default, SpreadLayout)]
pub struct Book {
    next_retirement_id: RetirementId,
    last_retirement_id: Option<RetirementId>,
    reports: StorageHashMap<RetirementId, Report>,
    account_mapping: StorageHashMap<AccountId, StorageBox<StorageVec<RetirementId>>>,
}

impl Book {
    pub fn take_next_retirement_id(&mut self) -> RetirementId {
        let next_retirement_id = self.next_retirement_id;
        self.next_retirement_id += 1;

        next_retirement_id
    }

    pub fn get_last_report_id(&self) -> Option<RetirementId> {
        self.last_retirement_id
    }

    pub fn get_report_by_id(&self, retirement_id: RetirementId) -> Result<Report, OperationError> {
        if !self.reports.contains_key(&retirement_id) {
            return Err(OperationError::RetirementReportNotFound);
        }

        Ok(self.reports.get(&retirement_id).unwrap().clone())
    }

    pub fn get_last_report(&self) -> Result<Report, OperationError> {
        if self.get_last_report_id().is_none() {
            return Err(OperationError::RetirementReportNotFound);
        }

        self.get_report_by_id(self.last_retirement_id.unwrap())
    }

    pub fn get_account_report(&self, account: AccountId) -> RetirementReports {
        let mut reports = GenericVec::new();

        if let Some(account_report_indices) = self.account_mapping.get(&account) {
            let account_report_indices = account_report_indices.iter();

            for report_id in account_report_indices {
                reports.push(self.reports.get(report_id).unwrap().clone());
            }
        }

        reports
    }

    pub fn insert_new_report(
        &mut self,
        account: AccountId,
        retirement_detail: &TokenBalanceDetail,
    ) -> Info {
        let next_retirement_id = self.take_next_retirement_id();
        let report = Report {
            id: next_retirement_id,
            block_number: block_number::<DefaultEnvironment>(),
            timestamp: block_timestamp::<DefaultEnvironment>(),
            beneficiary: account,
            token_id: retirement_detail.detail.id,
            amount: retirement_detail.balance,
            registry_id: retirement_detail.detail.registry_id.clone(),
        };
        self.last_retirement_id = Some(next_retirement_id);
        self.reports.insert(next_retirement_id, report);

        if !self.account_mapping.contains_key(&account) {
            self.account_mapping
                .insert(account, StorageBox::new(StorageVec::new()));
        }

        self.account_mapping
            .get_mut(&account)
            .unwrap()
            .push(next_retirement_id);

        Info {
            id: next_retirement_id,
            amount: retirement_detail.balance,
        }
    }
}
