pub mod resolver;
pub mod verifier;

use thiserror::Error;

pub struct FullClient;

#[derive(Error, Debug)]
pub enum TrustchainSovrinError {
    #[error("Could not build the request to did: {0}")]
    FailedToBuildRequest(String),

    #[error("Error while sending the query to ledger: {0}")]
    LedgerQuery(String),

    #[error("Could not parse ledger answer")]
    InvalidLedgerAnswer,

    #[error("The ledger answered the query with Failure: {0}")]
    QueryFailed(String),

    #[error("Conversion error between Indy and Trustchain formats")]
    CouldNotConvert,

}