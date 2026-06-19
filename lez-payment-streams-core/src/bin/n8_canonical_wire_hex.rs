use lez_payment_streams_core::{
    store_eligibility_canonical_payload, CanonicalStoreQueryParts, STORE_ELIGIBILITY_DOMAIN_PREFIX,
};

fn main() {
    // Empty message hashes - query returns all messages matching content topic
    let hashes: [[u8; 32]; 0] = [];
    // Content topic hashes to shard 1 with 8 shards (cluster 0)
    let topics = vec!["/lez-payment-streams/1/e2e-eligibility/proto".to_string()];
    let parts = CanonicalStoreQueryParts {
        request_id: "req-1",
        include_data: true,
        pubsub_topic: Some("/waku/2/rs/0/1"),  // cluster 0, shard 1
        content_topics: &topics,
        start_time: Some(10),
        end_time: None,
        message_hashes: &hashes,
        pagination_cursor: None,
        pagination_forward: true,
        pagination_limit: Some(100),
    };
    let body = store_eligibility_canonical_payload(&parts);
    let mut wire = STORE_ELIGIBILITY_DOMAIN_PREFIX.to_vec();
    wire.extend_from_slice(&body);
    print!("{}", hex::encode(wire));
}
