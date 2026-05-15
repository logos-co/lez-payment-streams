//! Stream folding facade and deterministic provider predicates (`integration-plan-v2.md` Step 3a).

mod predicates;

pub use predicates::{
    fold_stream, new_stream_satisfies_proposal, proposal_satisfies_policy,
    response_size_satisfies_policy, stream_satisfies_policy, unallocated_balance,
    StreamFoldedAtTime,
};
