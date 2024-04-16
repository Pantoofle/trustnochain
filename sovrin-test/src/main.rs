use futures_executor::block_on;
use indy_vdr::config::PoolConfig;
use indy_vdr::pool::{Pool, PoolBuilder, PoolTransactions, RequestResult};
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_vdr::resolver;
use indy_vdr::resolver::did_document::DidDocument;
use indy_vdr::resolver::types::DidDocumentMetadata;
use indy_vdr::resolver::utils::handle_internal_resolution_result;
use indy_vdr::utils::did::DidValue;

fn main() {

// Load genesis transactions. The corresponding transactions for the ledger you
// are connecting to should be saved to a local file.
    let txns = PoolTransactions::from_json_file("./mainNet.txn").unwrap();

// Create a PoolBuilder instance
    let pool_builder = PoolBuilder::new(PoolConfig::default(), txns);
// Convert into a thread-local Pool instance
    let pool = pool_builder.into_local().unwrap();


    // Create a GET_NYM request
    let request_builder = pool.get_request_builder();
    let target_did = DidValue::new("UFSFjGNiain5FQ2m88dijd", None);

    let request = request_builder.build_get_nym_request(None, &target_did, Some(29314), None).unwrap();
    let (result, _time) = block_on(perform_ledger_request(&pool, &request, None)).unwrap();

// Create a new GET_TXN request and dispatch it
//    let ledger_type = 1;  // 1 identifies the Domain ledger, see pool::LedgerType
//    let seq_no = 150;       // Transaction sequence number
//    let (result, _timing) = block_on(perform_get_txn(&pool, ledger_type, seq_no)).unwrap();

    let txn = match result{
        RequestResult::Failed(err) => return,
        RequestResult::Reply(data) => data,
    };

    let (res, meta) = handle_internal_resolution_result("sovrin", txn.as_str()).unwrap();

    if let resolver::types::Result::DidDocument(doc) = res{
        dbg!(&doc.to_value().unwrap());
    };
    if let resolver::types::Metadata::DidDocumentMetadata(meta) = meta{
        dbg!(&meta);
    }
}