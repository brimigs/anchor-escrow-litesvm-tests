use anchor_litesvm::{AnchorContext, tuple_args};
use litesvm::LiteSVM;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use solana_program_pack::Pack;

#[test]
fn test_make_with_improved_builder() {
    println!("\n🚀 Testing with Improved anchor-litesvm Builder API\n");

    // Initialize LiteSVM
    let mut svm = LiteSVM::new();

    // Deploy program
    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    svm.add_program(program_id, program_bytes);

    // Create AnchorContext
    let mut ctx = AnchorContext::new(svm, program_id);

    // Create and fund test accounts
    let maker = Keypair::new();
    ctx.svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();

    // Create token mints (still need manual setup for now)
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    setup_mints(&mut ctx, &maker, &mint_a, &mint_b);

    // Create and fund maker's ATA
    let maker_ata_a = get_associated_token_address(&maker.pubkey(), &mint_a.pubkey());
    setup_token_account(&mut ctx, &maker, &mint_a.pubkey(), &maker_ata_a, 1_000_000_000);

    // Calculate PDAs
    let seed: u64 = 42;
    let (escrow_pda, _) = ctx.find_pda(&[
        b"escrow",
        maker.pubkey().as_ref(),
        &seed.to_le_bytes(),
    ]);
    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // ✨ NEW IMPROVED API: Build instruction with fluent builder!
    let make_instruction = ctx
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
        .args(tuple_args((seed, 500_000_000u64, 1_000_000_000u64)))  // Tuple args - no struct needed!
        .build()
        .unwrap();

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
            println!("✅ Transaction succeeded with improved builder API!");
            println!("\n📝 Transaction logs:");
            for log in &res.logs {
                println!("  {}", log);
            }

            // Verify results
            verify_escrow_created(&ctx, &escrow_pda, &vault);

            println!("\n🎉 Test passed with improved anchor-litesvm builder!");
        }
        Err(e) => {
            panic!("❌ Transaction failed: {:?}", e);
        }
    }
}

#[test]
fn test_api_comparison() {
    println!("\n📊 API Evolution Comparison\n");

    println!("1️⃣ Original Manual Approach (test_make.rs):");
    println!("   - Define MakeArgs struct");
    println!("   - Implement BorshSerialize");
    println!("   - Calculate discriminator manually (5 lines)");
    println!("   - Serialize args manually (4 lines)");
    println!("   - Create AccountMeta vector (9+ lines)");
    println!("   Total: ~25+ lines\n");

    println!("2️⃣ First anchor-litesvm Version:");
    println!("   - Still need MakeArgs struct");
    println!("   - Still need manual AccountMeta vector");
    println!("   - But automatic discriminator");
    println!("   Total: ~15 lines\n");

    println!("3️⃣ NEW Improved Builder API:");
    println!("   - NO struct definition needed (use tuples!)");
    println!("   - Fluent account builder");
    println!("   - Named accounts for clarity");
    println!("   - Built-in system/token program helpers");
    println!("   Total: ~10 lines\n");

    println!("💡 Code Reduction: 60% less than original!");
    println!("💡 Much more readable and maintainable!");
}

// Helper functions
fn setup_mints(ctx: &mut AnchorContext, maker: &Keypair, mint_a: &Keypair, mint_b: &Keypair) {
    use litesvm_token::spl_token;

    let rent = ctx.svm.minimum_balance_for_rent_exemption(82);

    let instructions = vec![
        solana_sdk::system_instruction::create_account(
            &maker.pubkey(),
            &mint_a.pubkey(),
            rent,
            82,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint_a.pubkey(),
            &maker.pubkey(),
            None,
            9,
        ).unwrap(),
        solana_sdk::system_instruction::create_account(
            &maker.pubkey(),
            &mint_b.pubkey(),
            rent,
            82,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &mint_b.pubkey(),
            &maker.pubkey(),
            None,
            9,
        ).unwrap(),
    ];

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&maker.pubkey()),
        &[&maker, &mint_a, &mint_b],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
}

fn setup_token_account(
    ctx: &mut AnchorContext,
    owner: &Keypair,
    mint: &Pubkey,
    ata: &Pubkey,
    amount: u64,
) {
    use litesvm_token::spl_token;

    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &owner.pubkey(),
        &owner.pubkey(),
        mint,
        &spl_token::id(),
    );

    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        ata,
        &owner.pubkey(),
        &[],
        amount,
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix, mint_to_ix],
        Some(&owner.pubkey()),
        &[&owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
}

fn verify_escrow_created(ctx: &AnchorContext, escrow_pda: &Pubkey, vault: &Pubkey) {
    use litesvm_token::spl_token;

    let escrow_account = ctx.svm.get_account(escrow_pda);
    assert!(escrow_account.is_some(), "Escrow account should exist");
    println!("\n✅ Escrow account created at: {}", escrow_pda);

    let vault_account = ctx.svm.get_account(vault);
    assert!(vault_account.is_some(), "Vault account should exist");

    let vault_data = vault_account.unwrap();
    let vault_state = spl_token::state::Account::unpack(&vault_data.data).unwrap();
    assert_eq!(vault_state.amount, 1_000_000_000);
    println!("✅ Vault has {} tokens", vault_state.amount as f64 / 1_000_000_000.0);
}