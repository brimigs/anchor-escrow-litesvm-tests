use anchor_litesvm::{AnchorContext, tuple_args};
use litesvm::LiteSVM;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use solana_program_pack::Pack;
use litesvm_token::spl_token;

/// Complete escrow test demonstrating both make and take operations
/// using the improved anchor-litesvm builder API
#[test]
fn test_complete_escrow_flow() {
    println!("\nTesting Complete Escrow Flow: Make -> Take\n");

    // Initialize test environment
    let mut ctx = setup_test_environment();

    // Create participants
    let maker = Keypair::new();
    let taker = Keypair::new();

    // Fund accounts
    ctx.svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();
    ctx.svm.airdrop(&taker.pubkey(), 10_000_000_000).unwrap();
    println!("Funded maker and taker accounts");

    // Create token mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    create_mints(&mut ctx, &maker, &mint_a, &mint_b);
    println!("Created token mints A and B");

    // Setup initial token distributions
    // Maker gets 1000 tokens of mint_a (maker is authority of mint_a)
    let maker_ata_a = get_associated_token_address(&maker.pubkey(), &mint_a.pubkey());
    create_and_fund_token_account(&mut ctx, &maker, &mint_a.pubkey(), 1_000_000_000, &maker);

    // Taker gets 500 tokens of mint_b (maker is authority of mint_b too)
    let taker_ata_b = get_associated_token_address(&taker.pubkey(), &mint_b.pubkey());
    create_and_fund_token_account(&mut ctx, &taker, &mint_b.pubkey(), 500_000_000, &maker);

    println!("Initial token distribution complete:");
    println!("   Maker has 1000 tokens of mint_a");
    println!("   Taker has 500 tokens of mint_b");

    // === STEP 1: MAKE OFFER ===
    println!("\nStep 1: Maker creates escrow offer");

    let seed: u64 = 42;
    let (escrow_pda, _bump) = ctx.find_pda(&[
        b"escrow",
        maker.pubkey().as_ref(),
        &seed.to_le_bytes(),
    ]);
    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // Build make instruction with improved API
    let make_ix = ctx
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
        .args(tuple_args((
            seed,
            500_000_000u64,  // receive: 500 tokens of mint_b
            1_000_000_000u64, // deposit: 1000 tokens of mint_a
        )))
        .build()
        .unwrap();

    // Execute make instruction
    execute_instruction(&mut ctx, make_ix, &[&maker], "Make");

    // Verify escrow created
    verify_escrow_state(&ctx, &escrow_pda, &vault, 1_000_000_000);
    verify_token_balance(&ctx, &maker_ata_a, 0, "Maker's mint_a balance after escrow");

    // === STEP 2: TAKE OFFER ===
    println!("\nStep 2: Taker accepts escrow offer");

    // Prepare taker's accounts
    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &mint_a.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &mint_b.pubkey());

    // Build take instruction with improved API
    let take_ix = ctx
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
        .args(tuple_args(())) // Take has no args
        .build()
        .unwrap();

    // Execute take instruction with verbose logging
    std::env::set_var("VERBOSE", "1");
    execute_instruction(&mut ctx, take_ix, &[&taker], "Take");
    std::env::remove_var("VERBOSE");

    // === VERIFY FINAL STATE ===
    println!("\nVerifying final state after escrow completion:");

    // In LiteSVM, closed accounts might still exist with 0 lamports and 0 data
    // Check if escrow is effectively closed
    let escrow_closed = match ctx.svm.get_account(&escrow_pda) {
        None => true,
        Some(account) => account.lamports == 0 && account.data.is_empty(),
    };

    let vault_closed = match ctx.svm.get_account(&vault) {
        None => true,
        Some(account) => account.lamports == 0 && account.data.is_empty(),
    };

    assert!(escrow_closed, "Escrow should be closed (0 lamports, 0 data)");
    assert!(vault_closed, "Vault should be closed (0 lamports, 0 data)");
    println!("   Escrow and vault accounts closed");

    // Verify token transfers
    verify_token_balance(&ctx, &taker_ata_a, 1_000_000_000, "Taker's mint_a balance");
    verify_token_balance(&ctx, &maker_ata_b, 500_000_000, "Maker's mint_b balance");
    verify_token_balance(&ctx, &taker_ata_b, 0, "Taker's mint_b balance");

    println!("\nComplete escrow flow test passed!");
    println!("   Maker traded 1000 mint_a for 500 mint_b");
    println!("   Taker traded 500 mint_b for 1000 mint_a");
}

/// Test partial take scenario where taker doesn't have enough tokens
#[test]
fn test_take_insufficient_funds() {
    println!("\nTesting Take with Insufficient Funds\n");

    let mut ctx = setup_test_environment();

    // Setup accounts
    let maker = Keypair::new();
    let taker = Keypair::new();
    ctx.svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();
    ctx.svm.airdrop(&taker.pubkey(), 10_000_000_000).unwrap();

    // Create mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    create_mints(&mut ctx, &maker, &mint_a, &mint_b);

    // Maker has tokens, taker has insufficient tokens
    create_and_fund_token_account(&mut ctx, &maker, &mint_a.pubkey(), 1_000_000_000, &maker);
    create_and_fund_token_account(&mut ctx, &taker, &mint_b.pubkey(), 100_000_000, &maker); // Only 100, needs 500

    // Create escrow
    let seed: u64 = 99;
    let (escrow_pda, _) = ctx.find_pda(&[
        b"escrow",
        maker.pubkey().as_ref(),
        &seed.to_le_bytes(),
    ]);
    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());
    let maker_ata_a = get_associated_token_address(&maker.pubkey(), &mint_a.pubkey());

    // Make offer
    let make_ix = ctx
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
        .build()
        .unwrap();

    execute_instruction(&mut ctx, make_ix, &[&maker], "Make");

    // Attempt take with insufficient funds
    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &mint_a.pubkey());
    let taker_ata_b = get_associated_token_address(&taker.pubkey(), &mint_b.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &mint_b.pubkey());

    let take_ix = ctx
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
        .build()
        .unwrap();

    // Should fail due to insufficient funds
    let tx = Transaction::new_signed_with_payer(
        &[take_ix],
        Some(&taker.pubkey()),
        &[&taker],
        ctx.svm.latest_blockhash(),
    );

    match ctx.svm.send_transaction(tx) {
        Ok(_) => panic!("Transaction should have failed due to insufficient funds"),
        Err(_) => println!("Transaction correctly failed due to insufficient funds"),
    }

    // Verify escrow still exists
    assert!(ctx.svm.get_account(&escrow_pda).is_some(), "Escrow should still exist");
    verify_token_balance(&ctx, &vault, 1_000_000_000, "Vault should still have tokens");

    println!("Insufficient funds test passed!");
}

// === Helper Functions ===

fn setup_test_environment() -> AnchorContext {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    svm.add_program(program_id, program_bytes);
    AnchorContext::new(svm, program_id)
}

fn create_mints(ctx: &mut AnchorContext, authority: &Keypair, mint_a: &Keypair, mint_b: &Keypair) {
    let rent = ctx.svm.minimum_balance_for_rent_exemption(82);

    let instructions = vec![
        // Create mint A
        solana_sdk::system_instruction::create_account(
            &authority.pubkey(),
            &mint_a.pubkey(),
            rent,
            82,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint_a.pubkey(),
            &authority.pubkey(),
            None,
            9,
        ).unwrap(),
        // Create mint B
        solana_sdk::system_instruction::create_account(
            &authority.pubkey(),
            &mint_b.pubkey(),
            rent,
            82,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint_b.pubkey(),
            &authority.pubkey(),
            None,
            9,
        ).unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&authority.pubkey()),
        &[&authority, &mint_a, &mint_b],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
}

fn create_and_fund_token_account(
    ctx: &mut AnchorContext,
    owner: &Keypair,
    mint: &Pubkey,
    amount: u64,
    mint_authority: &Keypair,  // The authority that can mint tokens
) {
    let ata = get_associated_token_address(&owner.pubkey(), mint);

    // First create the ATA
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &owner.pubkey(),
        &owner.pubkey(),
        mint,
        &spl_token::id(),
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix],
        Some(&owner.pubkey()),
        &[&owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    // Then mint tokens using the mint authority
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        &ata,
        &mint_authority.pubkey(),
        &[],
        amount,
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&mint_authority.pubkey()),
        &[&mint_authority],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
}

fn execute_instruction(
    ctx: &mut AnchorContext,
    instruction: solana_sdk::instruction::Instruction,
    signers: &[&Keypair],
    name: &str,
) {
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&signers[0].pubkey()),
        signers,
        ctx.svm.latest_blockhash(),
    );

    match ctx.svm.send_transaction(tx) {
        Ok(res) => {
            println!("   {} instruction succeeded", name);
            if std::env::var("VERBOSE").is_ok() {
                for log in &res.logs {
                    println!("     {}", log);
                }
            }
        }
        Err(e) => panic!("{} instruction failed: {:?}", name, e),
    }
}

fn verify_escrow_state(ctx: &AnchorContext, escrow_pda: &Pubkey, vault: &Pubkey, expected_amount: u64) {
    let _escrow_account = ctx.svm.get_account(escrow_pda)
        .expect("Escrow account should exist");
    println!("   Escrow account created at: {}", escrow_pda);

    let vault_account = ctx.svm.get_account(vault)
        .expect("Vault account should exist");
    let vault_state = spl_token::state::Account::unpack(&vault_account.data).unwrap();
    assert_eq!(vault_state.amount, expected_amount);
    println!("   Vault has {} tokens", vault_state.amount as f64 / 1_000_000_000.0);
}

fn verify_token_balance(ctx: &AnchorContext, ata: &Pubkey, expected: u64, description: &str) {
    if let Some(account) = ctx.svm.get_account(ata) {
        let token_account = spl_token::state::Account::unpack(&account.data).unwrap();
        assert_eq!(
            token_account.amount, expected,
            "{} mismatch: expected {}, got {}",
            description, expected, token_account.amount
        );
        println!("   {}: {} tokens", description, token_account.amount as f64 / 1_000_000_000.0);
    } else if expected == 0 {
        // Account might not exist if balance should be 0
        println!("   {}: 0 tokens (account doesn't exist)", description);
    } else {
        panic!("{}: Account doesn't exist but expected {} tokens", description, expected);
    }
}