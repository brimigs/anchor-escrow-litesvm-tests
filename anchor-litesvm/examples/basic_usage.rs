/// Example showing how anchor-litesvm simplifies Anchor program testing
///
/// This example compares the traditional approach vs using anchor-litesvm

use anchor_litesvm::{AnchorContext, build_anchor_instruction};
use litesvm::LiteSVM;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use borsh::BorshSerialize;
use anchor_lang::AnchorSerialize;

// Example Anchor instruction arguments
#[derive(BorshSerialize)]
struct MakeArgs {
    seed: u64,
    receive: u64,
    amount: u64,
}

impl AnchorSerialize for MakeArgs {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(self, writer)
    }
}

fn main() {
    println!("=== Traditional Approach (without anchor-litesvm) ===\n");
    traditional_approach();

    println!("\n=== Using anchor-litesvm ===\n");
    using_anchor_litesvm();

    println!("\n=== Lines of Code Comparison ===");
    println!("Traditional: ~15 lines for instruction building");
    println!("With anchor-litesvm: 1 line for instruction building");
}

fn traditional_approach() {
    use sha2::{Digest, Sha256};

    let program_id = Pubkey::new_unique();

    // Manual discriminator calculation
    let mut hasher = Sha256::new();
    hasher.update(b"global:make");
    let hash = hasher.finalize();
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash[..8]);

    // Manual instruction data construction
    let args = MakeArgs {
        seed: 42,
        receive: 500_000_000,
        amount: 1_000_000_000,
    };

    let mut instruction_data = discriminator.to_vec();
    instruction_data.extend_from_slice(&args.seed.to_le_bytes());
    instruction_data.extend_from_slice(&args.receive.to_le_bytes());
    instruction_data.extend_from_slice(&args.amount.to_le_bytes());

    // Manual account meta setup
    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new(Pubkey::new_unique(), false),
            // ... many more accounts
        ],
        data: instruction_data,
    };

    println!("Built instruction with {} bytes of data", instruction.data.len());
}

fn using_anchor_litesvm() {
    // Initialize context
    let svm = LiteSVM::new();
    let program_id = Pubkey::new_unique();
    let ctx = AnchorContext::new(svm, program_id);

    // Build instruction with automatic discriminator - just 1 line!
    let instruction = ctx.build_instruction(
        "make",
        vec![
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new(Pubkey::new_unique(), false),
        ],
        MakeArgs {
            seed: 42,
            receive: 500_000_000,
            amount: 1_000_000_000,
        }
    ).unwrap();

    println!("Built instruction with {} bytes of data", instruction.data.len());

    // Also supports standalone instruction building
    let standalone_ix = build_anchor_instruction(
        &program_id,
        "make",
        vec![],
        MakeArgs { seed: 1, receive: 2, amount: 3 }
    ).unwrap();

    println!("Standalone instruction: {} bytes", standalone_ix.data.len());
}