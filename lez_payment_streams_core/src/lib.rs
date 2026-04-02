use serde::{Deserialize, Serialize};

#[cfg(test)]
mod vault_tests;

/// Example state struct — customize for your program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramState {
    pub initialized: bool,
    pub owner: [u8; 32],
}
