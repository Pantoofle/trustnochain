use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use ssi::did::Document;
use ssi::did_resolve::{ DocumentMetadata};
use serde::{Deserialize, Serialize};
use trustchain_core::resolver::{TrustchainResolver, ResolverError};
use trustchain_core::verifier::{VerifierError};

use crate::FullClient;
use crate::resolver::{SovrinResolver};


/// Data bundle for DID timestamp verification.
/// TODO: fill with what is needed for the verification.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerificationBundle {
    /// DID Document.
    did_doc: Document,
    did_doc_meta: DocumentMetadata,
}


pub struct TrustchainVerifier<U = FullClient>
{
    resolver: SovrinResolver,
    bundles: Mutex<HashMap<String, Arc<VerificationBundle>>>,
    _marker: PhantomData<U>,
}

impl TrustchainVerifier<FullClient>
{
    /// Constructs a new Sovrin Verifier.
    pub fn new(resolver: SovrinResolver) -> Self {
        let bundles = Mutex::new(HashMap::new());
        Self {
            resolver,
            bundles,
            _marker: PhantomData,
        }
    }

    /// Fetches the data needed to verify the DID's timestamp and stores it as a verification bundle.
    pub async fn fetch_bundle(&self, did: &str) -> Result<(), VerifierError> {
        let (did_doc, did_doc_meta) = self.resolve_did(did).await?;
        let bundle = VerificationBundle {
            did_doc,
            did_doc_meta,
        };
        // Insert the bundle into the HashMap of bundles, keyed by the DID.
        self.bundles
            .lock()
            .unwrap()
            .insert(did.to_string(), Arc::new(bundle));
        Ok(())
    }

    /// Gets a DID verification bundle, including a fetch if not initially cached.
    pub async fn verification_bundle(
        &self,
        did: &str,
    ) -> Result<Arc<VerificationBundle>, VerifierError> {
        // Fetch (and store) the bundle if it isn't already available.
        if !self.bundles.lock().unwrap().contains_key(did) {
            self.fetch_bundle(did).await?;
        }
        Ok(self.bundles.lock().unwrap().get(did).cloned().unwrap())
    }
    /// Resolves the given DID to obtain the DID Document and Document Metadata.
    async fn resolve_did(&self, did: &str) -> Result<(Document, DocumentMetadata), VerifierError> {
        let (res_meta, doc, doc_meta) = self.resolver.resolve_as_result(did).await?;
        if let (Some(doc), Some(doc_meta)) = (doc, doc_meta) {
            Ok((doc, doc_meta))
        } else {
            Err(VerifierError::DIDResolutionError(
                format!("Missing Document and/or DocumentMetadata for DID: {}", did),
                ResolverError::FailureWithMetadata(res_meta).into(),
            ))
        }
    }
}