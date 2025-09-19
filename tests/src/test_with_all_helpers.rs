use anchor_litesvm::{
    AnchorContext, AssertionHelpers, TestHelpers, tuple_args,
};
use litesvm::LiteSVM;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;

#[test]
fn test_complete_escrow_with_all_helpers() {
    println!("\nTesting Complete Escrow Flow with ALL New Helper Methods\n");

    // Initialize LiteSVM
    let mut svm = LiteSVM::new();

    // Deploy program
    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    svm.add_program(program_id, program_bytes);

    // Create AnchorContext
    let mut ctx = AnchorContext::new(svm, program_id);

    // === NEW: Use test helpers for account creation ===
    println!("Using test helpers for account creation...");

    // Create funded accounts in one line each!
    let maker = ctx.create_funded_account(10_000_000_000).unwrap();
    let taker = ctx.create_funded_account(10_000_000_000).unwrap();
    println!("Created and funded maker and taker accounts");

    // Create token mints in one line each!
    let mint_a = ctx.create_token_mint(&maker, 9).unwrap();
    let mint_b = ctx.create_token_mint(&maker, 9).unwrap();
    println!("Created token mints A and B");

    // Create token accounts and mint tokens in one line!
    let maker_ata_a = ctx.create_token_account(
        &maker,
        &mint_a.pubkey(),
        Some((1_000_000_000, &maker)), // Mint 1000 tokens
    ).unwrap();

    let taker_ata_b = ctx.create_token_account(
        &taker,
        &mint_b.pubkey(),
        Some((500_000_000, &maker)), // Mint 500 tokens
    ).unwrap();

    println!("Created and funded token accounts");

    // Calculate PDAs
    let seed: u64 = 42;
    let (escrow_pda, _) = ctx.find_pda(&[
        b"escrow",
        maker.pubkey().as_ref(),
        &seed.to_le_bytes(),
    ]);
    let vault = spl_associated_token_account::get_associated_token_address(
        &escrow_pda,
        &mint_a.pubkey(),
    );

    // === STEP 1: MAKE OFFER ===
    println!("\nStep 1: Maker creates escrow offer");

    // === NEW: Use execute method on builder for one-line execution! ===
    let make_result = ctx
        .instruction_builder("make")
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
        .unwrap();

    // === NEW: Use transaction result helpers ===
    make_result.assert_success();
    assert!(make_result.has_log("Instruction: Make"));
    println!("Make instruction succeeded with {} compute units", make_result.compute_units());

    // === NEW: Use assertion helpers ===
    ctx.assert_account_exists(&escrow_pda);
    ctx.assert_account_exists(&vault);
    ctx.assert_token_balance(&vault, 1_000_000_000);
    ctx.assert_token_balance(&maker_ata_a, 0);
    println!("Verified escrow state with assertion helpers");

    // === STEP 2: TAKE OFFER ===
    println!("\nStep 2: Taker accepts escrow offer");

    // Prepare taker's accounts
    let taker_ata_a = spl_associated_token_account::get_associated_token_address(
        &taker.pubkey(),
        &mint_a.pubkey(),
    );
    let maker_ata_b = spl_associated_token_account::get_associated_token_address(
        &maker.pubkey(),
        &mint_b.pubkey(),
    );

    // === NEW: Another one-line execution! ===
    let take_result = ctx
        .instruction_builder("take")
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
        .unwrap();

    take_result.assert_success();
    println!("Take instruction succeeded with {} compute units", take_result.compute_units());

    // === NEW: Use bulk assertion helpers ===
    println!("\nVerifying final state with assertion helpers:");

    // Assert accounts are closed
    ctx.assert_accounts_closed(&[&escrow_pda, &vault]);
    println!("   Escrow and vault accounts properly closed");

    // Assert final token balances
    ctx.assert_token_balance_with_msg(&taker_ata_a, 1_000_000_000, "Taker should have 1000 mint_a");
    ctx.assert_token_balance_with_msg(&maker_ata_b, 500_000_000, "Maker should have 500 mint_b");
    ctx.assert_token_balance_with_msg(&taker_ata_b, 0, "Taker should have 0 mint_b");

    println!("\nComplete escrow flow test passed with all new helpers!");
    println!("Code reduction: ~70% compared to original approach!");
}

#[test]
fn test_transaction_helpers() {
    println!("\nDemonstrating Transaction Helper Methods\n");

    let mut svm = LiteSVM::new();
    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    svm.add_program(program_id, include_bytes!("../../target/deploy/anchor_escrow.so"));

    let mut ctx = AnchorContext::new(svm, program_id);

    // Create test accounts
    let accounts = ctx.create_funded_accounts(3, 5_000_000_000).unwrap();
    println!("Created 3 funded accounts in one call");

    // Batch airdrop to multiple accounts
    let keypair1 = solana_sdk::signature::Keypair::new();
    let keypair2 = solana_sdk::signature::Keypair::new();
    ctx.batch_airdrop(&[&keypair1.pubkey(), &keypair2.pubkey()], 1_000_000_000).unwrap();
    println!("Batch airdropped to multiple accounts");

    // Test lamports assertion
    ctx.assert_account_lamports(&accounts[0].pubkey(), 5_000_000_000);

    println!("Transaction helpers test passed!");
}

#[test]
fn test_improved_dx_comparison() {
    println!("\nDX Improvement Comparison\n");

    println!("OLD WAY (25+ lines):");
    println!("- Manual discriminator calculation");
    println!("- Manual transaction building");
    println!("- Manual error handling");
    println!("- Manual token account setup");
    println!("- Manual assertion logic\n");

    println!("NEW WAY (5-10 lines):");
    println!("- ctx.create_funded_account(amount)");
    println!("- ctx.create_token_mint(&authority, decimals)");
    println!("- ctx.create_token_account(&owner, &mint, Some((amount, &authority)))");
    println!("- instruction_builder.execute(&mut ctx, &[&signer])");
    println!("- ctx.assert_token_balance(&account, expected)");
    println!("- result.assert_success()");

    println!("\nKey improvements:");
    println!("- 70% less boilerplate code");
    println!("- Chainable, fluent API");
    println!("- Built-in error handling");
    println!("- Automatic transaction construction");
    println!("- Reusable test utilities");
}