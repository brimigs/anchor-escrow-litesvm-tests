//! # anchor-litesvm
//!
//! A lightweight testing utility library that bridges Anchor and LiteSVM,
//! dramatically simplifying the process of testing Anchor programs.
//!
//! ## Features
//!
//! - **Automatic Instruction Building**: Handles discriminator calculation and data serialization
//! - **Type-Safe Account Deserialization**: Deserialize Anchor accounts with proper type handling
//! - **Direct LiteSVM Access**: Full control over the underlying LiteSVM instance
//! - **Minimal Overhead**: Thin wrapper that doesn't hide functionality
//!
//! ## Quick Start
//!
//! ```no_run
//! use anchor_litesvm::AnchorContext;
//! use litesvm::LiteSVM;
//! use solana_program::pubkey::Pubkey;
//!
//! // Initialize LiteSVM
//! let mut svm = LiteSVM::new();
//! let program_id = Pubkey::new_unique();
//! // svm.add_program(program_id, &program_bytes);
//!
//! // Create Anchor context
//! let ctx = AnchorContext::new(svm, program_id);
//!
//! // Use ctx.build_instruction() for automatic discriminator handling
//! // Use ctx.get_anchor_account() for type-safe deserialization
//! // Access ctx.svm directly for any LiteSVM operations
//! ```

pub mod account;
pub mod context;
pub mod instruction;
pub mod instruction_builder;

// Re-export main types for convenience
pub use account::{get_anchor_account, get_anchor_account_unchecked, AccountError};
pub use context::AnchorContext;
pub use instruction::{build_anchor_instruction, calculate_anchor_discriminator};
pub use instruction_builder::{InstructionBuilder, tuple_args, TupleArgs};

// Re-export commonly used external types
pub use litesvm::LiteSVM;
pub use solana_program::instruction::{AccountMeta, Instruction};
pub use solana_program::pubkey::Pubkey;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use anchor_lang::AnchorSerialize;
    use borsh::BorshSerialize;

    #[test]
    fn test_full_workflow() {
        // Create test context
        let svm = LiteSVM::new();
        let program_id = Pubkey::new_unique();
        let ctx = AnchorContext::new(svm, program_id);

        // Test instruction building
        #[derive(BorshSerialize)]
        struct TestArgs {
            value: u64,
        }

        impl AnchorSerialize for TestArgs {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                BorshSerialize::serialize(self, writer)
            }
        }

        let accounts = vec![
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ];

        let instruction = ctx
            .build_instruction("test", accounts, TestArgs { value: 42 })
            .unwrap();

        assert_eq!(instruction.program_id, program_id);
        assert!(!instruction.data.is_empty());
    }
}