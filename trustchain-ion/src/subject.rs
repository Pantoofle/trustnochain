use did_ion::sidetree::Sidetree;
use did_ion::ION;
use ssi::did::Document;
use ssi::{jwk::JWK, one_or_many::OneOrMany};
use trustchain_core::{
    key_manager::{KeyManager, KeyManagerError, SubjectKeyManager},
    subject::{Subject, SubjectError},
};

pub struct IONSubject {
    did: String,
    signing_keys: Option<OneOrMany<JWK>>,
}

impl SubjectKeyManager for IONSubject {}

impl KeyManager for IONSubject {}

impl IONSubject {
    /// Construct a new TrustchainSubject instance.
    pub fn new(did: &str) -> Self {
        Self {
            did: did.to_owned(),
            signing_keys: None,
        }
    }

    /// Gets the signing keys.
    fn signing_keys(&mut self) -> Result<OneOrMany<JWK>, KeyManagerError> {
        match self.signing_keys.as_mut() {
            Some(keys) => Ok(keys.clone()),
            None => {
                // self.read_signing_keys(&self.did)?;
                let signing_keys = self.read_signing_keys(&self.did)?;
                self.signing_keys = Some(signing_keys.clone());
                Ok(signing_keys)
            }
        }
    }

    // fn load(&mut self, did: &str) -> Result<(), KeyManagerError> {
    //     if let Ok(signing_keys) = self.read_signing_keys(did) {
    //         self.signing_keys = Some(signing_keys);
    //         Ok(())
    //     } else {
    //         Err(KeyManagerError::FailedToLoadKey)
    //     }
    // }
}

type SubjectData = (String, OneOrMany<JWK>);

impl From<SubjectData> for IONSubject {
    fn from(subject_data: SubjectData) -> Self {
        IONSubject {
            did: subject_data.0,
            signing_keys: Some(subject_data.1),
        }
    }
}

impl Subject for IONSubject {
    fn did(&self) -> &str {
        &self.did
    }

    fn attest(&self, doc: &Document, signing_key: &JWK) -> Result<String, SubjectError> {
        let algorithm = ION::SIGNATURE_ALGORITHM;

        let canonical_document = match ION::json_canonicalization_scheme(&doc) {
            Ok(str) => str,
            Err(_) => return Err(SubjectError::InvalidDocumentParameters(doc.id.clone())),
        };
        let proof = (&doc.id.clone(), canonical_document);

        let proof_json = match ION::json_canonicalization_scheme(&proof) {
            Ok(str) => str,
            Err(_) => return Err(SubjectError::InvalidDocumentParameters(doc.id.clone())),
        };

        let proof_json_bytes = ION::hash(proof_json.as_bytes());

        match ssi::jwt::encode_sign(algorithm, &proof_json_bytes, signing_key) {
            Ok(str) => Ok(str),
            Err(e) => Err(SubjectError::SigningError(doc.id.clone(), e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;
    use ssi::did::Document;

    use trustchain_core::data::{TEST_SIGNING_KEYS, TEST_TRUSTCHAIN_DOCUMENT};

    // // Set-up tempdir and use as env var for TRUSTCHAIN_DATA
    // // https://stackoverflow.com/questions/58006033/how-to-run-setup-code-before-any-tests-run-in-rust
    // static INIT: Once = Once::new();
    // pub fn init() {
    //     INIT.call_once(|| {
    //         // initialization code here
    //         let tempdir = tempfile::tempdir().unwrap();
    //         std::env::set_var(TRUSTCHAIN_DATA, Path::new(tempdir.as_ref().as_os_str()));
    //     });
    // }

    #[test]
    fn test_from() -> Result<(), Box<dyn std::error::Error>> {
        let did = "did:ion:test:EiCBr7qGDecjkR2yUBhn3aNJPUR3TSEOlkpNcL0Q5Au9YP";
        let keys: OneOrMany<JWK> = serde_json::from_str(TEST_SIGNING_KEYS)?;

        let target = IONSubject::from((did.to_string(), keys.clone()));

        assert_eq!(target.did(), did);
        assert_eq!(target.signing_keys.unwrap(), keys);

        Ok(())
    }

    #[test]
    fn test_attest() -> Result<(), Box<dyn std::error::Error>> {
        let did = "did:ion:test:EiCBr7qGDecjkR2yUBhn3aNJPUR3TSEOlkpNcL0Q5Au9YP";
        let keys: OneOrMany<JWK> = serde_json::from_str(TEST_SIGNING_KEYS)?;
        let signing_key = keys.first().unwrap();

        let target = IONSubject::from((did.to_string(), keys.clone()));

        println!("{:?}", target.read_signing_keys(did));

        let doc = Document::from_json(TEST_TRUSTCHAIN_DOCUMENT).expect("Document failed to load.");

        let result = target.attest(&doc, signing_key);
        assert!(result.is_ok());

        let proof_result = result?;

        // Test that the proof_result string is valid JSON.
        // TODO: figure out the correct result type here (guessed &str).
        let json_proof_result: Result<&str, serde_json::Error> =
            serde_json::from_str(&proof_result);

        // TODO: check for a key-value in the JSON.
        // println!("{:?}", json_proof_result);
        Ok(())
    }

    // #[test]
    // fn test_signing_keys() {}

    // #[test]
    // fn test_load() {}

    // #[test]
    // fn test_save() {}

    // #[test]
    // fn test_get_public_key() {}

    // #[test]
    // fn test_generate_signing_keys() {}
}
