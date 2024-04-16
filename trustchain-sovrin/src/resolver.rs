use async_trait::async_trait;
use ssi::did_resolve::{DIDResolver, DocumentMetadata, ResolutionInputMetadata, ResolutionMetadata};
use futures_executor::block_on;
use indy_vdr::config::PoolConfig;
use indy_vdr::pool::{SharedPool, Pool, PoolBuilder, PoolTransactions, RequestResult};
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_vdr::resolver::did_document::DidDocument;
use indy_vdr::resolver::types::DidDocumentMetadata;
use indy_vdr::resolver::types::Result as ResolverResult;
use indy_vdr::resolver::types::Metadata;
use indy_vdr::resolver::utils::handle_internal_resolution_result;
use indy_vdr::utils::did::DidValue;
use serde_json;
use ssi::did::Document;
use trustchain_core::resolver::{ TrustchainResolver};
use crate::TrustchainSovrinError;

pub struct Transaction;

pub struct SovrinResolver {
    pool: SharedPool
}

impl SovrinResolver {
    pub fn new() -> Self {
        // Load genesis transactions. The corresponding transactions for the ledger you
        // are connecting to should be saved to a local file.
        // TODO Put the root transaction path in a config file
        let txns = PoolTransactions::from_json_file("./mainNet.txn").unwrap();
        // Create a PoolBuilder instance
        let pool_builder = PoolBuilder::new(PoolConfig::default(), txns);
        // Convert into a thread-local Pool instance
        let pool = pool_builder.into_shared().unwrap();

        Self {
            pool
        }
    }

    pub fn fetch_did(&self, did: &str) -> Result<(DidDocument, DidDocumentMetadata), TrustchainSovrinError> {
        // Create a GET_NYM request
        let request_builder = &self.pool.get_request_builder();
        let target_did = DidValue::new(did, None);
        let request = request_builder.build_get_nym_request(None, &target_did, None, None)
            .map_err(|_| TrustchainSovrinError::FailedToBuildRequest(did.into()))?;

        // Run the request
        let (ledger_answer, _time) = block_on(perform_ledger_request(&self.pool, &request, None))
            .map_err(|e| TrustchainSovrinError::LedgerQuery(e.to_string()))?;

        // Parse the result
        let (res, meta) = match ledger_answer.map_result(|txn|
            handle_internal_resolution_result("sovrin", txn.as_str())
        ).map_err(|_| TrustchainSovrinError::InvalidLedgerAnswer)? {
            RequestResult::Reply((res, meta)) => Ok((res, meta)),
            RequestResult::Failed(err) => Err(TrustchainSovrinError::QueryFailed(err.to_string()))
        }?;

        // Check that we got a DID and not another type of document. Return the DID and Metadata
        match (res, meta){
            (ResolverResult::DidDocument(doc), Metadata::DidDocumentMetadata(meta)) => Ok((doc, meta)),
            (_, _) =>  Err(TrustchainSovrinError::InvalidLedgerAnswer),
        }
    }
}

fn convert_resolution_documents(doc: DidDocument, meta: DidDocumentMetadata) -> Result<(Document, DocumentMetadata), TrustchainSovrinError>{
    // TODO: Adapt if some specific data must be moved from Doc to Meta
    let meta : DocumentMetadata = serde_json::from_value(
        serde_json::to_value(&meta).map_err(|_| TrustchainSovrinError::CouldNotConvert)?
    ).map_err(|_| TrustchainSovrinError::CouldNotConvert)?;

    let doc : Document = serde_json::from_value(
        doc.to_value().map_err(|_| TrustchainSovrinError::CouldNotConvert)?
    ).map_err(|_| TrustchainSovrinError::CouldNotConvert)?;

    Ok((doc, meta))
}

#[async_trait]
impl DIDResolver for SovrinResolver{
    async fn resolve(&self, did: &str, _input_metadata: &ResolutionInputMetadata) -> (ResolutionMetadata, Option<Document>, Option<DocumentMetadata>) {
        self.fetch_did(did)
            .and_then(|(doc, meta)| convert_resolution_documents(doc, meta))
            .map_or_else(
                |e| (
                    ResolutionMetadata{ error: Some(format!("Error while resolving DID : {}", e.to_string())), content_type: None, property_set : None },
                    None,
                    None
                ),
                |(doc, meta) | (
                    ResolutionMetadata { error: None, content_type: None, property_set: None },
                    Some(doc),
                    Some(meta)
                )
            )
    }
}

#[async_trait]
impl TrustchainResolver for SovrinResolver{
    fn wrapped_resolver(&self) -> &dyn DIDResolver {
        self
    }
}