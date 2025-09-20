use anchor_litesvm::{
    AnchorLiteSVM, AssertionHelpers, TestHelpers, tuple_args,
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use spl_associated_token_account::get_associated_token_address;

#[test]
fn test_escrow_with_anchor_litesvm() {
git 
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