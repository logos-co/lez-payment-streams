//! PDA helpers for tests (same algorithm as `spel-framework-core::pda`), avoiding a second
//! `nssa_core` revision from SPEL's tagged dependency.

use nssa_core::account::AccountId;
use nssa_core::program::{PdaSeed, ProgramId};
use sha2::{Digest, Sha256};

pub fn seed_from_str(s: &str) -> [u8; 32] {
    let src = s.as_bytes();
    assert!(src.len() <= 32, "seed string '{}' exceeds 32 bytes", s);
    let mut bytes = [0u8; 32];
    bytes[..src.len()].copy_from_slice(src);
    bytes
}

pub fn compute_pda(program_id: &ProgramId, seeds: &[&[u8; 32]]) -> AccountId {
    assert!(!seeds.is_empty(), "PDA requires at least one seed");

    let combined = if seeds.len() == 1 {
        *seeds[0]
    } else {
        let mut hasher = Sha256::new();
        for seed in seeds {
            hasher.update(seed);
        }
        hasher.finalize().into()
    };

    let pda_seed = PdaSeed::new(combined);
    AccountId::from((program_id, &pda_seed))
}
