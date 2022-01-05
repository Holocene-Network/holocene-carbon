use scale::{Decode, Encode};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Message {
    BlockchainCorrupted,
    CannotTransferZeroCarbonUnit,
    CustodianAlreadyRegistered,
    CustodianNotFound,
    InsufficientCarbonUnit,
    RetirementReportNotFound,
    TokenAlreadyMinted,
    TokenMintRequestAlreadyPending,
    TokenMintRequestNotFound,
    TokenNotFound,
    Unauthorized,
}
