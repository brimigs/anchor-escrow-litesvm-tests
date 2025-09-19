# anchor-litesvm

A lightweight testing utility library that bridges Anchor and LiteSVM, dramatically simplifying the process of testing Anchor programs.

## Problem

Testing Anchor programs with LiteSVM currently requires significant boilerplate:
- Manual instruction discriminator calculation using SHA256
- Verbose instruction data serialization
- Complex account meta setup
- Manual account deserialization with proper type handling

## Solution

`anchor-litesvm` provides a minimal wrapper around LiteSVM that handles Anchor-specific patterns, reducing test code by 60-70% while maintaining full control and flexibility.

## MVP Features (Current)

### 1. Automatic Instruction Building
Eliminates ~15 lines of boilerplate per instruction:

```rust
// Before (manual approach):
let mut hasher = Sha256::new();
hasher.update(b"global:make");
let hash = hasher.finalize();
let mut discriminator = [0u8; 8];
discriminator.copy_from_slice(&hash[..8]);

let mut instruction_data = discriminator.to_vec();
instruction_data.extend_from_slice(&seed.to_le_bytes());
instruction_data.extend_from_slice(&receive.to_le_bytes());
instruction_data.extend_from_slice(&amount.to_le_bytes());

let instruction = Instruction {
    program_id,
    accounts: vec![/* ... many lines of AccountMeta ... */],
    data: instruction_data,
};

// After (with anchor-litesvm):
let ix = ctx.build_instruction(
    "make",
    accounts,
    MakeArgs { seed, receive, amount }
);
```

### 2. Type-Safe Account Deserialization
Automatic Anchor account unpacking:

```rust
// Before:
let account_data = svm.get_account(&escrow_pda).unwrap();
let escrow: EscrowState = EscrowState::try_from_slice(&account_data.data[8..]).unwrap();

// After:
let escrow: EscrowState = ctx.get_anchor_account(&escrow_pda)?;
```

### 3. Direct LiteSVM Access
The `AnchorContext` provides full access to the underlying LiteSVM instance:

```rust
let mut ctx = AnchorContext::new(svm, program_id);
// Direct access for any LiteSVM operations
ctx.svm.airdrop(&pubkey, amount);
ctx.svm.send_transaction(tx);
```

## Usage Example

```rust
use anchor_litesvm::AnchorContext;
use litesvm::LiteSVM;

#[test]
fn test_anchor_program() {
    // Initialize LiteSVM as normal
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, program_bytes);

    // Create Anchor context
    let ctx = AnchorContext::new(svm, program_id);

    // Build instruction with automatic discriminator
    let ix = ctx.build_instruction(
        "initialize",
        vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(state_pda, false),
        ],
        InitializeArgs { amount: 100 }
    )?;

    // Send transaction using LiteSVM
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx)?;

    // Deserialize Anchor account
    let state: StateAccount = ctx.get_anchor_account(&state_pda)?;
    assert_eq!(state.amount, 100);
}
```

## Roadmap

### Phase 1: MVP (Current)
- ✅ Automatic instruction discriminator calculation
- ✅ Instruction data serialization with Borsh
- ✅ Type-safe account deserialization
- ✅ Simple context wrapper maintaining LiteSVM access

### Phase 2: Enhanced Instruction Building
- [ ] IDL file parsing for automatic account resolution
- [ ] Instruction builder with method chaining
- [ ] Automatic signer detection
- [ ] Better error messages with context

### Phase 3: Testing Utilities
- [ ] PDA derivation helpers matching Anchor patterns
- [ ] Account state assertions
- [ ] Transaction result parsing
- [ ] Event emission testing

### Phase 4: Advanced Features
- [ ] Program deployment from source
- [ ] Workspace management for multi-program testing
- [ ] Time manipulation helpers
- [ ] Account snapshot/rollback for test isolation

### Phase 5: Developer Experience
- [ ] Procedural macros for test setup
- [ ] Integration with anchor-client types
- [ ] Comprehensive examples and documentation
- [ ] Performance optimizations

## Design Principles

1. **Minimal API Surface** - Keep it simple and focused
2. **No Hidden Magic** - Direct access to LiteSVM, no functionality hidden
3. **Composable** - Works alongside litesvm-token and other utilities
4. **Type Safety** - Leverage Rust's type system for correctness
5. **Zero Overhead** - Thin wrapper, no performance penalties

## Comparison with Alternatives

| Feature | Raw LiteSVM | anchor-test | anchor-litesvm |
|---------|-------------|-------------|----------------|
| Setup Complexity | High | Medium | Low |
| Anchor Integration | Manual | Full | Targeted |
| Performance | Fastest | Slower | Fast |
| Flexibility | Full | Limited | Full |
| Lines of Code | ~200 | ~100 | ~80 |

## Contributing

This is an MVP focused on the most painful parts of Anchor + LiteSVM integration. Contributions are welcome, especially for:
- IDL parsing improvements
- Additional test utilities
- Documentation and examples
- Performance optimizations

## License

MIT OR Apache-2.0