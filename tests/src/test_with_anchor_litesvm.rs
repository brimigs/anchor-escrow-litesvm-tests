use anchor_litesvm::AnchorContext;
use litesvm::LiteSVM;
use solana_sdk::{
    instruction::AccountMeta,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use borsh::BorshSerialize;
use anchor_lang::AnchorSerialize;
use solana_program_pack::Pack;

// Define the instruction arguments using Borsh
#[derive(Debug, BorshSerialize)]
struct MakeArgs {
    seed: u64,
    receive: u64,
    amount: u64,
}

// Implement AnchorSerialize for our args
impl AnchorSerialize for MakeArgs {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(self, writer)
    }
}

#[test]
fn test_make_simplified() {
    println!("\nTesting with anchor-litesvm - Simplified Version\n");

    // Initialize LiteSVM
    let mut svm = LiteSVM::new();

    // Deploy program
    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    svm.add_program(program_id, program_bytes);

    // Create AnchorContext - the key simplification!
    let mut ctx = AnchorContext::new(svm, program_id);

    // Create and fund test accounts
    let maker = Keypair::new();
    ctx.svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();

    // Create token mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    // Setup mints using litesvm-token
    use litesvm_token::spl_token;

    // Create and initialize mints (same as before)
    let rent = ctx.svm.minimum_balance_for_rent_exemption(82);
    let create_mint_a_account_ix = solana_sdk::system_instruction::create_account(
        &maker.pubkey(),
        &mint_a.pubkey(),
        rent,
        82,
        &spl_token::id(),
    );
    let create_mint_b_account_ix = solana_sdk::system_instruction::create_account(
        &maker.pubkey(),
        &mint_b.pubkey(),
        rent,
        82,
        &spl_token::id(),
    );

    let init_mint_a_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_a.pubkey(),
        &maker.pubkey(),
        None,
        9,
    ).unwrap();

    let init_mint_b_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_b.pubkey(),
        &maker.pubkey(),
        None,
        9,
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_mint_a_account_ix, init_mint_a_ix, create_mint_b_account_ix, init_mint_b_ix],
        Some(&maker.pubkey()),
        &[&maker, &mint_a, &mint_b],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    // Create maker's ATA for mint_a
    let maker_ata_a = get_associated_token_address(&maker.pubkey(), &mint_a.pubkey());
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &maker.pubkey(),
        &maker.pubkey(),
        &mint_a.pubkey(),
        &spl_token::id(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix],
        Some(&maker.pubkey()),
        &[&maker],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    // Mint tokens
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_a.pubkey(),
        &maker_ata_a,
        &maker.pubkey(),
        &[],
        1_000_000_000,
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&maker.pubkey()),
        &[&maker],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    // Calculate PDAs using the helper
    let seed: u64 = 42;
    let (escrow_pda, _bump) = ctx.find_pda(&[
        b"escrow",
        maker.pubkey().as_ref(),
        &seed.to_le_bytes(),
    ]);

    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // ✨ THE MAGIC: Build instruction with automatic discriminator!
    // This single line replaces ~15 lines of manual discriminator calculation and serialization
    let make_instruction = ctx.build_instruction(
        "make",  // Just the instruction name - discriminator is calculated automatically!
        vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new_readonly(mint_a.pubkey(), false),
            AccountMeta::new_readonly(mint_b.pubkey(), false),
            AccountMeta::new(maker_ata_a, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        MakeArgs {
            seed,
            receive: 500_000_000,
            amount: 1_000_000_000,
        },
    ).unwrap();

    // Send transaction
    let tx = Transaction::new_signed_with_payer(
        &[make_instruction],
        Some(&maker.pubkey()),
        &[&maker],
        ctx.svm.latest_blockhash(),
    );

    let result = ctx.svm.send_transaction(tx);

    match result {
        Ok(res) => {
            println!("Transaction succeeded with anchor-litesvm!");
            println!("\n📝 Transaction logs:");
            for log in &res.logs {
                println!("  {}", log);
            }

            // Verify escrow account
            let escrow_account = ctx.svm.get_account(&escrow_pda);
            assert!(escrow_account.is_some(), "Escrow account should exist");
            println!("\nEscrow account created at: {}", escrow_pda);

            // Verify vault account
            let vault_account = ctx.svm.get_account(&vault);
            assert!(vault_account.is_some(), "Vault account should exist");

            // Check token balance
            let vault_data = vault_account.unwrap();
            let vault_state = spl_token::state::Account::unpack(&vault_data.data).unwrap();
            assert_eq!(vault_state.amount, 1_000_000_000);
            println!("Vault has {} tokens", vault_state.amount as f64 / 1_000_000_000.0);

            println!("\nTest passed with anchor-litesvm!");
        }
        Err(e) => {
            panic!("❌ Transaction failed: {:?}", e);
        }
    }
}

#[test]
fn test_comparison() {
    println!("\n📊 Code Comparison: Traditional vs anchor-litesvm\n");
    println!("Traditional approach (test_make.rs):");
    println!("  - Lines for discriminator: ~5");
    println!("  - Lines for serialization: ~4");
    println!("  - Lines for instruction building: ~6");
    println!("  - Total: ~15 lines\n");

    println!("With anchor-litesvm:");
    println!("  - Lines for instruction: 1");
    println!("  - Total: 1 line");
    println!("\n💡 Reduction: 93% less code for instruction building!");

    println!("\nKey benefits:");
    println!("  - Automatic discriminator calculation");
    println!("  - Clean, readable API");
    println!("  - Type-safe with Anchor serialization");
    println!("  - Direct access to LiteSVM for flexibility");
    println!("  - PDA calculation helpers");
}