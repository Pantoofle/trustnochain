use async_trait::async_trait;
use did_ion::sidetree::DocumentState;
use futures::{stream, StreamExt, TryStreamExt};
use ssi::{
    did_resolve::DIDResolver,
    ldp::LinkedDataDocument,
    vc::{Credential, CredentialOrJWT, URI},
    vc::{LinkedDataProofOptions, Presentation},
};
use std::error::Error;
use trustchain_core::{
    chain::DIDChain,
    holder::Holder,
    issuer::{Issuer, IssuerError},
    resolver::ResolverResult,
    vc::CredentialError,
    verifier::{Timestamp, Verifier, VerifierError},
    vp::PresentationError,
};
use trustchain_ion::{
    attest::attest_operation, attestor::IONAttestor, create::create_operation, get_ion_resolver,
    verifier::IONVerifier,
};

use crate::TrustchainAPI;

/// API for Trustchain CLI DID functionality.
#[async_trait]
pub trait TrustchainDIDAPI {
    /// Creates a controlled DID from a passed document state, writing the associated create operation
    /// to file in the operations path returning the file name including the created DID suffix.
    // TODO: make specific error?
    fn create(
        document_state: Option<DocumentState>,
        verbose: bool,
    ) -> Result<String, Box<dyn Error>> {
        create_operation(document_state, verbose)
    }
    /// An uDID attests to a dDID, writing the associated update operation to file in the operations
    /// path.
    // TODO: make pecific error?
    async fn attest(did: &str, controlled_did: &str, verbose: bool) -> Result<(), Box<dyn Error>> {
        attest_operation(did, controlled_did, verbose).await
    }
    /// Resolves a given DID using given endpoint.
    async fn resolve(did: &str, endpoint: &str) -> ResolverResult {
        // main_resolve(did, verbose)
        let resolver = get_ion_resolver(endpoint);

        // Result metadata, Document, Document metadata
        resolver.resolve_as_result(did).await
    }

    /// Verifies a given DID using a resolver available at given endpoint, returning a result.
    async fn verify(
        did: &str,
        root_event_time: Timestamp,
        endpoint: &str,
    ) -> Result<DIDChain, VerifierError> {
        IONVerifier::new(get_ion_resolver(endpoint))
            .verify(did, root_event_time)
            .await
    }

    // // TODO: the below have no CLI implementation currently but are planned
    // /// Generates an update operation and writes to operations path.
    // fn update(did: &str, controlled_did: &str, verbose: bool) -> Result<(), Box<dyn Error>> {
    //     todo!()
    // }
    // /// Generates a recover operation and writes to operations path.
    // fn recover(did: &str, verbose: bool) -> Result<(), Box<dyn Error>> {
    //     todo!()
    // }
    // /// Generates a deactivate operation and writes to operations path.
    // fn deactivate(did: &str, verbose: bool) -> Result<(), Box<dyn Error>> {
    //     todo!()
    // }
    // /// Publishes operations within the operations path (queue).
    // fn publish(did: &str, verbose: bool) -> Result<(), Box<dyn Error>> {
    //     todo!()
    // }
}

/// API for Trustchain CLI VC functionality.
#[async_trait]
pub trait TrustchainVCAPI {
    /// Signs a credential.
    async fn sign(
        mut credential: Credential,
        did: &str,
        key_id: Option<&str>,
        endpoint: &str,
    ) -> Result<Credential, IssuerError> {
        let resolver = get_ion_resolver(endpoint);
        credential.issuer = Some(ssi::vc::Issuer::URI(URI::String(did.to_string())));
        let attestor = IONAttestor::new(did);
        attestor.sign(&credential, key_id, &resolver).await
    }

    /// Verifies a credential and returns a `DIDChain` if valid.
    async fn verify_credential<T: DIDResolver + Send + Sync>(
        credential: &Credential,
        ldp_options: Option<LinkedDataProofOptions>,
        root_event_time: Timestamp,
        verifier: &IONVerifier<T>,
    ) -> Result<DIDChain, CredentialError> {
        // Verify signature
        let result = credential.verify(ldp_options, verifier.resolver()).await;
        if !result.errors.is_empty() {
            return Err(CredentialError::VerificationResultError(result));
        }
        // Verify issuer
        let issuer = credential
            .get_issuer()
            .ok_or(CredentialError::NoIssuerPresent)?;
        Ok(verifier.verify(issuer, root_event_time).await?)
    }
}
#[async_trait]
pub trait TrustchainVPAPI {
    /// As a holder issue a verifiable presentation.
    async fn sign_presentation(
        presentation: Presentation,
        did: &str,
        key_id: Option<&str>,
        endpoint: &str,
    ) -> Result<Presentation, PresentationError> {
        let resolver = get_ion_resolver(endpoint);
        let attestor = IONAttestor::new(did);
        Ok(attestor
            .sign_presentation(&presentation, key_id, &resolver)
            .await?)
    }
    // TODO: verify holder's signature
    /// Verifies a verifiable presentation. Analogous with [didkit](https://docs.rs/didkit/latest/didkit/c/fn.didkit_vc_verify_presentation.html).
    async fn verify_presentation<T: DIDResolver + Send + Sync>(
        presentation: &Presentation,
        ldp_options: Option<LinkedDataProofOptions>,
        root_event_time: Timestamp,
        verifier: &IONVerifier<T>,
    ) -> Result<(), PresentationError> {
        let credentials = presentation
            .verifiable_credential
            .as_ref()
            .ok_or(PresentationError::NoCredentialsPresent)?;

        // https://gendignoux.com/blog/2021/04/01/rust-async-streams-futures-part1.html#unordered-buffering-1
        // https://docs.rs/futures-util/latest/futures_util/stream/trait.TryStreamExt.html#method.try_for_each_concurrent
        // TODO consider concurrency limit (as rate limiting for verifier requests)
        let limit = Some(5);
        let ldp_options_vec: Vec<Option<LinkedDataProofOptions>> = (0..credentials.len())
            .map(|_| ldp_options.clone())
            .collect();

        stream::iter(credentials.into_iter().zip(ldp_options_vec))
            .map(Ok)
            .try_for_each_concurrent(limit, |(credential_or_jwt, ldp_options)| async move {
                match credential_or_jwt {
                    CredentialOrJWT::Credential(credential) => {
                        println!("start");
                        let v = TrustchainAPI::verify_credential(
                            credential,
                            ldp_options,
                            root_event_time,
                            verifier,
                        )
                        .await
                        .map(|_| ())
                        .map_err(|err| err.into());
                        println!("done");
                        v
                    }

                    CredentialOrJWT::JWT(jwt) => {
                        let result =
                            Credential::verify_jwt(jwt, ldp_options, verifier.resolver()).await;
                        if !result.errors.is_empty() {
                            Err(PresentationError::CredentialError(
                                CredentialError::VerificationResultError(result),
                            ))
                        } else {
                            Ok(())
                        }
                    }
                }
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::api::{TrustchainVCAPI, TrustchainVPAPI};
    use crate::TrustchainAPI;
    use ssi::one_or_many::OneOrMany;
    use ssi::vc::{Context, Contexts, Credential, CredentialOrJWT, JWTClaims, Presentation, URI};
    use trustchain_core::issuer::Issuer;
    use trustchain_ion::attestor::IONAttestor;
    use trustchain_ion::get_ion_resolver;
    use trustchain_ion::verifier::IONVerifier;

    // The root event time of DID documents in `trustchain-ion/src/data.rs` used for unit tests and the test below.
    const ROOT_EVENT_TIME_1: u32 = 1666265405;

    const TEST_UNSIGNED_VC: &str = r##"{
        "@context": [
          "https://www.w3.org/2018/credentials/v1",
          "https://www.w3.org/2018/credentials/examples/v1",
          "https://w3id.org/citizenship/v1"
        ],
        "credentialSchema": {
          "id": "did:example:cdf:35LB7w9ueWbagPL94T9bMLtyXDj9pX5o",
          "type": "did:example:schema:22KpkXgecryx9k7N6XN1QoN3gXwBkSU8SfyyYQG"
        },
        "type": ["VerifiableCredential"],
        "issuer": "did:ion:test:EiAtHHKFJWAk5AsM3tgCut3OiBY4ekHTf66AAjoysXL65Q",
        "image": "some_base64_representation",
        "credentialSubject": {
          "givenName": "Jane",
          "familyName": "Doe",
          "degree": {
            "type": "BachelorDegree",
            "name": "Bachelor of Science and Arts",
            "college": "College of Engineering"
          }
        }
      }
      "##;

    #[tokio::test]
    async fn test_verify_credential() {
        let vc_with_proof = signed_credential().await;
        let resolver = get_ion_resolver("http://localhost:3000/");
        let res = TrustchainAPI::verify_credential(
            &vc_with_proof,
            None,
            ROOT_EVENT_TIME_1,
            &IONVerifier::new(resolver),
        )
        .await;
        assert!(res.is_ok());
    }
    #[tokio::test]
    async fn test_verify_presentation() {
        let vc_with_proof = signed_credential().await;
        let resolver = get_ion_resolver("http://localhost:3000/");
        let presentation = Presentation {
            context: Contexts::One(Context::URI(URI::String(String::from("test")))),
            holder: None,
            verifiable_credential: Some(OneOrMany::Many(vec![
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof.clone()),
                CredentialOrJWT::Credential(vc_with_proof),
            ])),
            id: None,
            type_: OneOrMany::One(String::from("test")),
            proof: None,
            property_set: None,
        };
        let res = TrustchainAPI::verify_presentation(
            &presentation,
            None,
            ROOT_EVENT_TIME_1,
            &IONVerifier::new(resolver),
        )
        .await;
        assert!(res.is_ok());
    }

    async fn signed_credential() -> Credential {
        // 1. Set-up
        let did = "did:ion:test:EiAtHHKFJWAk5AsM3tgCut3OiBY4ekHTf66AAjoysXL65Q";
        // Make resolver
        let resolver = get_ion_resolver("http://localhost:3000/");
        // 2. Load Attestor
        let attestor = IONAttestor::new(did);
        // 3. Read credential
        let vc: Credential = serde_json::from_str(TEST_UNSIGNED_VC).unwrap();
        // Use attest_credential method instead of generating and adding proof
        attestor.sign(&vc, None, &resolver).await.unwrap()
    }
}
