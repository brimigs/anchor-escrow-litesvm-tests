use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use borsh::BorshSerialize;
use sha2::{Digest, Sha256};
use solana_program_pack::Pack;

#[derive(Debug, BorshSerialize)]
struct MakeArgs {
    seed: u64,
    receive: u64,
    amount: u64,
}

#[test]
fn test_my_program() {
    // Initialize the test environment
    let mut svm = LiteSVM::new();

    // Deploy your program
    let program_id = Pubkey::from_str_const("8LTee82TkoqBoBjBAz2yAAKSj9ckr7zz5vMi6rJQTwhJ");
    let program_bytes = include_bytes!("../../target/deploy/anchor_escrow.so");
    svm.add_program(program_id, program_bytes);

    // Create and fund test accounts
    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();

    // Create two token mints
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    // Use litesvm-token to create mints
    use litesvm_token::spl_token;

    // Create mint A
    let create_mint_a_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_a.pubkey(),
        &maker.pubkey(),
        None,
        9, // decimals
    ).unwrap();

    // Create mint B
    let create_mint_b_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_b.pubkey(),
        &maker.pubkey(),
        None,
        9, // decimals
    ).unwrap();

    // First create the mint accounts
    let rent = svm.minimum_balance_for_rent_exemption(82);
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

    // Create mints transaction
    let tx = Transaction::new_signed_with_payer(
        &[create_mint_a_account_ix, create_mint_a_ix, create_mint_b_account_ix, create_mint_b_ix],
        Some(&maker.pubkey()),
        &[&maker, &mint_a, &mint_b],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Create maker's associated token account for mint_a
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
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Mint tokens to maker's ATA
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_a.pubkey(),
        &maker_ata_a,
        &maker.pubkey(),
        &[],
        1_000_000_000, // 1 token with 9 decimals
    ).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[mint_to_ix],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Calculate PDAs and addresses
    let seed: u64 = 42;
    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &program_id,
    );

    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    // Build instruction discriminator using Anchor's standard method
    let mut hasher = Sha256::new();
    hasher.update(b"global:make");
    let hash = hasher.finalize();
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash[..8]);

    // Serialize instruction arguments
    let args = MakeArgs {
        seed,
        receive: 500_000_000, // 0.5 tokens
        amount: 1_000_000_000, // 1 token
    };

    let mut instruction_data = discriminator.to_vec();
    instruction_data.extend_from_slice(&seed.to_le_bytes());
    instruction_data.extend_from_slice(&args.receive.to_le_bytes());
    instruction_data.extend_from_slice(&args.amount.to_le_bytes());

    // Build the make instruction with all required accounts
    let make_instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),  // maker
            AccountMeta::new(escrow_pda, false),      // escrow
            AccountMeta::new_readonly(mint_a.pubkey(), false), // mint_a
            AccountMeta::new_readonly(mint_b.pubkey(), false), // mint_b
            AccountMeta::new(maker_ata_a, false),     // maker_ata_a
            AccountMeta::new(vault, false),           // vault
            AccountMeta::new_readonly(spl_associated_token_account::id(), false), // associated_token_program
            AccountMeta::new_readonly(spl_token::id(), false), // token_program
            AccountMeta::new_readonly(system_program::id(), false), // system_program
        ],
        data: instruction_data,
    };

    // Build and send transaction
    let tx = Transaction::new_signed_with_payer(
        &[make_instruction],
        Some(&maker.pubkey()),
        &[&maker],
        svm.latest_blockhash(),
    );

    // Execute and verify
    let result = svm.send_transaction(tx);

    match result {
        Ok(res) => {
            println!("Transaction succeeded!");

            for log in &res.logs {
                println!("  {}", log);
            }

            // Verify escrow account was created
            let escrow_account = svm.get_account(&escrow_pda);
            assert!(escrow_account.is_some(), "Escrow account should exist");
            println!("Escrow account created at: {}", escrow_pda);

            // Verify vault account was created and has tokens
            let vault_account = svm.get_account(&vault);
            assert!(vault_account.is_some(), "Vault account should exist");
            println!("Vault account created at: {}", vault);

            // Check token balances
            use litesvm_token::spl_token;
            let vault_data = vault_account.unwrap();
            let vault_state = spl_token::state::Account::unpack(&vault_data.data).unwrap();
            assert_eq!(vault_state.amount, 1_000_000_000, "Vault should have 1 token");
            println!("Vault has {} tokens", vault_state.amount as f64 / 1_000_000_000.0);

            let maker_ata_data = svm.get_account(&maker_ata_a).unwrap();
            let maker_ata_state = spl_token::state::Account::unpack(&maker_ata_data.data).unwrap();
            assert_eq!(maker_ata_state.amount, 0, "Maker ATA should have 0 tokens after transfer");
            println!("Maker ATA has {} tokens (after transfer)", maker_ata_state.amount);
        }
        Err(e) => {
            panic!("Transaction failed: {:?}", e);
        }
    }
}