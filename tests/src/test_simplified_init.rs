use anchor_litesvm::{
    AnchorLiteSVM, AssertionHelpers, TestHelpers, tuple_args,
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use spl_associated_token_account::get_associated_token_address;

#[test]
fn test_with_simplified_initialization() {
    println!("\nTesting with Simplified AnchorLiteSVM Initialization\n");

    // OLD WAY (4-5 lines):
    // let mut svm = LiteSVM::new();
    // let program_id = Pubkey::from_str_const("...");
    // let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    // svm.add_program(program_id, program_bytes);
    // let mut ctx = AnchorContext::new(svm, program_id);

    // NEW WAY (1 line!):
    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);

    println!("Context created with one line!");

    // Now use all the helper methods as before
    let maker = ctx.create_funded_account(10_000_000_000).unwrap();
    let taker = ctx.create_funded_account(10_000_000_000).unwrap();

    let mint_a = ctx.create_token_mint(&maker, 9).unwrap();
    let mint_b = ctx.create_token_mint(&maker, 9).unwrap();

    let maker_ata_a = ctx.create_token_account(
        &maker,
        &mint_a.pubkey(),
        Some((1_000_000_000, &maker)),
    ).unwrap();

    let _taker_ata_b = ctx.create_token_account(
        &taker,
        &mint_b.pubkey(),
        Some((500_000_000, &maker)),
    ).unwrap();

    // Calculate PDAs
    let seed: u64 = 42;
    let (escrow_pda, _) = ctx.find_pda(&[
        b"escrow",
        maker.pubkey().as_ref(),
        &seed.to_le_bytes(),
    ]);
    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // Execute make instruction
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

    make_result.assert_success();
    ctx.assert_account_exists(&escrow_pda);
    ctx.assert_token_balance(&vault, 1_000_000_000);

    println!("\nTest passed with simplified initialization!");
}

#[test]
fn test_builder_patterns() {
    println!("\nTesting Different Builder Patterns\n");

    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");

    // Pattern 1: Direct convenience method
    println!("Pattern 1: Direct convenience method");
    let _ctx1 = AnchorLiteSVM::build_with_program(program_id, program_bytes);
    println!("  Created with build_with_program()");

    // Pattern 2: Builder pattern
    println!("\nPattern 2: Builder pattern");
    let _ctx2 = AnchorLiteSVM::new()
        .deploy_program(program_id, program_bytes)
        .build();
    println!("  Created with builder pattern");

    // Pattern 3: With primary program specification
    println!("\nPattern 3: With primary program specification");
    let _ctx3 = AnchorLiteSVM::new()
        .deploy_program(program_id, program_bytes)
        .with_primary_program(program_id)
        .build();
    println!("  Created with primary program specified");

    println!("\nAll builder patterns work correctly!");
}

#[test]
fn test_extension_trait() {
    use anchor_litesvm::ProgramTestExt;

    println!("\nTesting Extension Trait for Even Simpler Init\n");

    // Using the extension trait on Pubkey directly
    const PROGRAM_ID: Pubkey = Pubkey::new_from_array([
        141, 28, 143, 49, 34, 46, 168, 14, 16, 16, 32, 66, 43, 113, 121, 0,
        72, 240, 216, 91, 93, 37, 70, 133, 51, 175, 86, 206, 162, 197, 3, 41
    ]); // Same as "8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ"

    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");

    // Super clean one-liner using extension trait
    let mut ctx = PROGRAM_ID.test_with(program_bytes);

    // Quick smoke test
    let account = ctx.create_funded_account(1_000_000_000).unwrap();
    ctx.assert_account_lamports(&account.pubkey(), 1_000_000_000);

    println!("Extension trait initialization works perfectly!");
}

#[test]
fn test_api_evolution() {
    println!("\nAPI Evolution Summary\n");

    println!("ORIGINAL (5+ lines):");
    println!("  let mut svm = LiteSVM::new();");
    println!("  let program_id = Pubkey::from_str_const(...);");
    println!("  let program_bytes = include_bytes!(...);");
    println!("  svm.add_program(program_id, program_bytes);");
    println!("  let mut ctx = AnchorContext::new(svm, program_id);");

    println!("\nIMPROVED (1 line):");
    println!("  let mut ctx = AnchorLiteSVM::build_with_program(program_id, program_bytes);");

    println!("\nOR WITH EXTENSION TRAIT:");
    println!("  let mut ctx = PROGRAM_ID.test_with(program_bytes);");

    println!("\nBenefit: 80% less boilerplate for test setup!");
}