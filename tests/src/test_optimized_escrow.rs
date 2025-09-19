use anchor_litesvm::{
    AnchorLiteSVM, AssertionHelpers, TestHelpers, tuple_args,
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use spl_associated_token_account::get_associated_token_address;

/// Ultra-optimized escrow test using ALL anchor-litesvm features
/// This demonstrates the absolute minimal code needed for comprehensive testing
#[test]
fn test_optimized_complete_escrow() {
    // 1-line initialization!
    let mut ctx = AnchorLiteSVM::build_with_program(
        Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ"),
        include_bytes!("../../target/deploy/anchor_escrow.so"),
    );

    // Create ALL test accounts in just 4 lines!
    let maker = ctx.create_funded_account(10_000_000_000).unwrap();
    let taker = ctx.create_funded_account(10_000_000_000).unwrap();
    let mint_a = ctx.create_token_mint(&maker, 9).unwrap();
    let mint_b = ctx.create_token_mint(&maker, 9).unwrap();

    // Create and fund token accounts in 2 lines!
    let maker_ata_a = ctx.create_token_account(&maker, &mint_a.pubkey(), Some((1_000_000_000, &maker))).unwrap();
    let taker_ata_b = ctx.create_token_account(&taker, &mint_b.pubkey(), Some((500_000_000, &maker))).unwrap();

    // PDAs
    let seed = 42u64;
    let (escrow_pda, _) = ctx.find_pda(&[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()]);
    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // MAKE: Build and execute in one expression!
    ctx.instruction_builder("make")
        .signer("maker", &maker)
        .account_mut("escrow", escrow_pda)
        .account("mint_a", mint_a.pubkey())
        .account("mint_b", mint_b.pubkey())
        .account_mut("maker_ata_a", maker_ata_a)
        .account_mut("vault", vault)
        .associated_token_program()
        .token_program()
        .system_program()
        .args(tuple_args((seed, 500_000_000u64, 1_000_000_000u64)))
        .execute(&mut ctx, &[&maker])
        .unwrap()
        .assert_success();

    // Verify make with one-line assertions
    ctx.assert_account_exists(&escrow_pda);
    ctx.assert_token_balance(&vault, 1_000_000_000);
    ctx.assert_token_balance(&maker_ata_a, 0);

    // TAKE: Another one-liner execution!
    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &mint_a.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &mint_b.pubkey());

    ctx.instruction_builder("take")
        .signer("taker", &taker)
        .account_mut("maker", maker.pubkey())
        .account_mut("escrow", escrow_pda)
        .account("mint_a", mint_a.pubkey())
        .account("mint_b", mint_b.pubkey())
        .account_mut("vault", vault)
        .account_mut("taker_ata_a", taker_ata_a)
        .account_mut("taker_ata_b", taker_ata_b)
        .account_mut("maker_ata_b", maker_ata_b)
        .associated_token_program()
        .token_program()
        .system_program()
        .args(tuple_args(()))
        .execute(&mut ctx, &[&taker])
        .unwrap()
        .assert_success();

    // Final verification in 3 lines!
    ctx.assert_accounts_closed(&[&escrow_pda, &vault]);
    ctx.assert_token_balance(&taker_ata_a, 1_000_000_000);
    ctx.assert_token_balance(&maker_ata_b, 500_000_000);
}

/// Even more concise test with helper function
#[test]
fn test_ultra_minimal_escrow() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ"),
        include_bytes!("../../target/deploy/anchor_escrow.so"),
    );

    // Setup everything in one function call
    let (maker, taker, mint_a, mint_b, escrow_pda, vault) = setup_escrow_test(&mut ctx);

    // Execute make
    execute_make(&mut ctx, &maker, mint_a.pubkey(), mint_b.pubkey(), escrow_pda, vault);

    // Execute take
    execute_take(&mut ctx, &maker, &taker, &mint_a, &mint_b, escrow_pda, vault);

    // Verify final state
    ctx.assert_accounts_closed(&[&escrow_pda, &vault]);
}

/// Test with error handling
#[test]
fn test_insufficient_funds_optimized() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ"),
        include_bytes!("../../target/deploy/anchor_escrow.so"),
    );

    // Quick setup with insufficient funds
    let maker = ctx.create_funded_account(10_000_000_000).unwrap();
    let taker = ctx.create_funded_account(10_000_000_000).unwrap();
    let mint_a = ctx.create_token_mint(&maker, 9).unwrap();
    let mint_b = ctx.create_token_mint(&maker, 9).unwrap();

    ctx.create_token_account(&maker, &mint_a.pubkey(), Some((1_000_000_000, &maker))).unwrap();
    ctx.create_token_account(&taker, &mint_b.pubkey(), Some((100_000_000, &maker))).unwrap(); // Only 100!

    let seed = 99u64;
    let (escrow_pda, _) = ctx.find_pda(&[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()]);
    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // Make succeeds
    ctx.instruction_builder("make")
        .signer("maker", &maker)
        .account_mut("escrow", escrow_pda)
        .account("mint_a", mint_a.pubkey())
        .account("mint_b", mint_b.pubkey())
        .account_mut("maker_ata_a", get_associated_token_address(&maker.pubkey(), &mint_a.pubkey()))
        .account_mut("vault", vault)
        .associated_token_program()
        .token_program()
        .system_program()
        .args(tuple_args((seed, 500_000_000u64, 1_000_000_000u64)))
        .execute(&mut ctx, &[&maker])
        .unwrap()
        .assert_success();

    // Take should fail - using match for cleaner error handling
    let take_result = ctx.instruction_builder("take")
        .signer("taker", &taker)
        .account_mut("maker", maker.pubkey())
        .account_mut("escrow", escrow_pda)
        .account("mint_a", mint_a.pubkey())
        .account("mint_b", mint_b.pubkey())
        .account_mut("vault", vault)
        .account_mut("taker_ata_a", get_associated_token_address(&taker.pubkey(), &mint_a.pubkey()))
        .account_mut("taker_ata_b", get_associated_token_address(&taker.pubkey(), &mint_b.pubkey()))
        .account_mut("maker_ata_b", get_associated_token_address(&maker.pubkey(), &mint_b.pubkey()))
        .associated_token_program()
        .token_program()
        .system_program()
        .args(tuple_args(()))
        .execute(&mut ctx, &[&taker]);

    // Clean assertion of failure
    assert!(take_result.is_err(), "Take should fail with insufficient funds");

    // Verify escrow still exists
    ctx.assert_account_exists(&escrow_pda);
    ctx.assert_token_balance(&vault, 1_000_000_000);
}

/// Batch operations test - demonstrates efficiency
#[test]
fn test_batch_escrow_operations() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ"),
        include_bytes!("../../target/deploy/anchor_escrow.so"),
    );

    // Create multiple accounts in batch
    let accounts = ctx.create_funded_accounts(5, 10_000_000_000).unwrap();
    let maker = &accounts[0];
    let taker = &accounts[1];

    // Create mints
    let mint_a = ctx.create_token_mint(maker, 9).unwrap();
    let mint_b = ctx.create_token_mint(maker, 9).unwrap();

    // Create token accounts once
    let maker_ata_a = get_associated_token_address(&maker.pubkey(), &mint_a.pubkey());
    let taker_ata_b = get_associated_token_address(&taker.pubkey(), &mint_b.pubkey());
    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &mint_a.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &mint_b.pubkey());

    // Create and fund the ATAs with enough tokens for all escrows
    ctx.create_token_account(maker, &mint_a.pubkey(), Some((3_000_000_000, maker))).unwrap();
    ctx.create_token_account(taker, &mint_b.pubkey(), Some((1_500_000_000, maker))).unwrap();

    // Test multiple escrows with different seeds
    for seed in [1u64, 2, 3] {

        let (escrow_pda, _) = ctx.find_pda(&[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()]);
        let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

        // Make
        ctx.instruction_builder("make")
            .signer("maker", maker)
            .account_mut("escrow", escrow_pda)
            .account("mint_a", mint_a.pubkey())
            .account("mint_b", mint_b.pubkey())
            .account_mut("maker_ata_a", maker_ata_a)
            .account_mut("vault", vault)
            .associated_token_program()
            .token_program()
            .system_program()
            .args(tuple_args((seed, 500_000_000u64, 1_000_000_000u64)))
            .execute(&mut ctx, &[maker])
            .unwrap();

        // Take
        ctx.instruction_builder("take")
            .signer("taker", taker)
            .account_mut("maker", maker.pubkey())
            .account_mut("escrow", escrow_pda)
            .account("mint_a", mint_a.pubkey())
            .account("mint_b", mint_b.pubkey())
            .account_mut("vault", vault)
            .account_mut("taker_ata_a", taker_ata_a)
            .account_mut("taker_ata_b", taker_ata_b)
            .account_mut("maker_ata_b", maker_ata_b)
            .associated_token_program()
            .token_program()
            .system_program()
            .args(tuple_args(()))
            .execute(&mut ctx, &[taker])
            .unwrap();

        // Verify cleanup for each escrow
        ctx.assert_account_closed(&escrow_pda);
        ctx.assert_account_closed(&vault);
    }
}

// Helper functions for ultra-minimal test
fn setup_escrow_test(ctx: &mut anchor_litesvm::AnchorContext) -> (
    solana_sdk::signature::Keypair,
    solana_sdk::signature::Keypair,
    solana_sdk::signature::Keypair,
    solana_sdk::signature::Keypair,
    Pubkey,
    Pubkey,
) {
    let maker = ctx.create_funded_account(10_000_000_000).unwrap();
    let taker = ctx.create_funded_account(10_000_000_000).unwrap();
    let mint_a = ctx.create_token_mint(&maker, 9).unwrap();
    let mint_b = ctx.create_token_mint(&maker, 9).unwrap();

    ctx.create_token_account(&maker, &mint_a.pubkey(), Some((1_000_000_000, &maker))).unwrap();
    ctx.create_token_account(&taker, &mint_b.pubkey(), Some((500_000_000, &maker))).unwrap();

    let seed = 42u64;
    let (escrow_pda, _) = ctx.find_pda(&[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()]);
    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    (maker, taker, mint_a, mint_b, escrow_pda, vault)
}

fn execute_make(
    ctx: &mut anchor_litesvm::AnchorContext,
    maker: &solana_sdk::signature::Keypair,
    mint_a: Pubkey,
    mint_b: Pubkey,
    escrow_pda: Pubkey,
    vault: Pubkey,
) {
    ctx.instruction_builder("make")
        .signer("maker", maker)
        .account_mut("escrow", escrow_pda)
        .account("mint_a", mint_a)
        .account("mint_b", mint_b)
        .account_mut("maker_ata_a", get_associated_token_address(&maker.pubkey(), &mint_a))
        .account_mut("vault", vault)
        .associated_token_program()
        .token_program()
        .system_program()
        .args(tuple_args((42u64, 500_000_000u64, 1_000_000_000u64)))
        .execute(ctx, &[maker])
        .unwrap()
        .assert_success();
}

fn execute_take(
    ctx: &mut anchor_litesvm::AnchorContext,
    maker: &solana_sdk::signature::Keypair,
    taker: &solana_sdk::signature::Keypair,
    mint_a: &solana_sdk::signature::Keypair,
    mint_b: &solana_sdk::signature::Keypair,
    escrow_pda: Pubkey,
    vault: Pubkey,
) {
    ctx.instruction_builder("take")
        .signer("taker", taker)
        .account_mut("maker", maker.pubkey())
        .account_mut("escrow", escrow_pda)
        .account("mint_a", mint_a.pubkey())
        .account("mint_b", mint_b.pubkey())
        .account_mut("vault", vault)
        .account_mut("taker_ata_a", get_associated_token_address(&taker.pubkey(), &mint_a.pubkey()))
        .account_mut("taker_ata_b", get_associated_token_address(&taker.pubkey(), &mint_b.pubkey()))
        .account_mut("maker_ata_b", get_associated_token_address(&maker.pubkey(), &mint_b.pubkey()))
        .associated_token_program()
        .token_program()
        .system_program()
        .args(tuple_args(()))
        .execute(ctx, &[taker])
        .unwrap()
        .assert_success();
}