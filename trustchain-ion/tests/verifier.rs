use serde_json::json;
use trustchain_core::commitment::Commitment;
use trustchain_core::verifier::{Timestamp, Verifier};
use trustchain_ion::get_ion_resolver;
use trustchain_ion::verifier::IONVerifier;

#[test]
#[ignore] // Requires a running Sidetree node listening on http://localhost:3000.
fn trustchain_verification() {
    // Integration test of the Trustchain resolution pipeline.
    // root - root-plus-1 - root-plus-2
    let dids = vec![
        "did:ion:test:EiCClfEdkTv_aM3UnBBhlOV89LlGhpQAbfeZLFdFxVFkEg",
        "did:ion:test:EiBVpjUxXeSRJpvj2TewlX9zNF3GKMCKWwGmKBZqF6pk_A",
        "did:ion:test:EiAtHHKFJWAk5AsM3tgCut3OiBY4ekHTf66AAjoysXL65Q",
    ];

    // Construct a Trustchain Resolver from a Sidetree (ION) DIDMethod.
    let resolver = get_ion_resolver("http://localhost:3000/");
    let verifier = IONVerifier::new(resolver);
    for did in dids {
        // TODO.
        // let result = verifier.verify(did, ROOT_EVENT_TIME);
        // println!(
        //     "DID: {:?},  VERIFIED!!!\n{:?}",
        //     did,
        //     result.as_ref().unwrap()
        // );
        // assert!(result.is_ok());
    }
}

#[test]
#[ignore = "Integration test requires ION, Bitcoin RPC & IPFS"]
fn test_verifiable_timestamp() {
    let resolver = get_ion_resolver("http://localhost:3000/");
    let mut target = IONVerifier::new(resolver);

    let did = "did:ion:test:EiCClfEdkTv_aM3UnBBhlOV89LlGhpQAbfeZLFdFxVFkEg";
    let result = target.verifiable_timestamp(did);

    assert!(result.is_ok());

    let verifiable_timestamp = result.unwrap();

    // Check that the DID commitment is the expected proof of work hash.
    // See https://blockstream.info/testnet/block/000000000000000eaa9e43748768cd8bf34f43aaa03abd9036c463010a0c6e7f
    let expected_hash = "000000000000000eaa9e43748768cd8bf34f43aaa03abd9036c463010a0c6e7f";
    assert_eq!(verifiable_timestamp.hash().unwrap(), expected_hash);

    // Check that the DID timestamp is correct by comparing to the known header.
    assert_eq!(verifiable_timestamp.timestamp(), 1666265405 as Timestamp);

    // Confirm that the same timestamp is the expected data in the TimestampCommitment.
    assert_eq!(
        verifiable_timestamp.timestamp_commitment().expected_data(),
        &json!(1666265405)
    );

    // Verify the timestamp.
    let actual = target.verify_timestamp(&verifiable_timestamp);

    if let Err(e) = actual {
        println!("{:?}", e);
        panic!()
    }
}
