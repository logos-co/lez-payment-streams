//! Stream folding facade and deterministic provider predicates (`docs/plan/index.md` Step 3a).

mod predicates;

pub use predicates::{
    create_stream_deadline_satisfies_policy_as_of, fold_stream, new_stream_satisfies_proposal,
    proposal_satisfies_policy, response_within_policy, stream_satisfies_policy,
    unallocated_balance, StreamFoldedAtTime,
};
