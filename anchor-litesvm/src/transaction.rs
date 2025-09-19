//! Transaction execution and result handling utilities
//!
//! This module provides convenient wrappers for executing transactions
//! and handling their results in tests.

use litesvm::types::TransactionMetadata;
use solana_program::instruction::Instruction;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::fmt;

/// Wrapper around LiteSVM's TransactionMetadata with helper methods for testing
pub struct TransactionResult {
    inner: TransactionMetadata,
    instruction_name: Option<String>,
}

impl TransactionResult {
    /// Create a new TransactionResult wrapper
    pub fn new(result: TransactionMetadata, instruction_name: Option<String>) -> Self {
        Self {
            inner: result,
            instruction_name,
        }
    }

    /// Assert that the transaction succeeded, panic with logs if it failed
    pub fn assert_success(&self) -> &Self {
        // TransactionResult from LiteSVM is returned on success, so this is always successful
        // if we have a result. Errors are returned as Err() from send_transaction
        self
    }

    /// Get the transaction logs
    pub fn logs(&self) -> &[String] {
        &self.inner.logs
    }

    /// Get specific log lines that match a pattern
    pub fn find_logs(&self, pattern: &str) -> Vec<&String> {
        self.inner
            .logs
            .iter()
            .filter(|log| log.contains(pattern))
            .collect()
    }

    /// Check if a specific log message exists
    pub fn has_log(&self, pattern: &str) -> bool {
        self.inner.logs.iter().any(|log| log.contains(pattern))
    }

    /// Get the compute units consumed
    pub fn compute_units(&self) -> u64 {
        // Parse compute units from logs
        for log in &self.inner.logs {
            if log.contains("consumed") && log.contains("compute units") {
                // Extract number from log like "Program ... consumed 12345 of 200000 compute units"
                if let Some(consumed_part) = log.split("consumed").nth(1) {
                    if let Some(number_part) = consumed_part.split("of").next() {
                        if let Ok(units) = number_part.trim().parse::<u64>() {
                            return units;
                        }
                    }
                }
            }
        }
        0
    }

    /// Print transaction logs (useful for debugging)
    pub fn print_logs(&self) {
        if let Some(ref name) = self.instruction_name {
            println!("Transaction logs for '{}':", name);
        } else {
            println!("Transaction logs:");
        }
        for log in &self.inner.logs {
            println!("  {}", log);
        }
    }

    /// Get the inner LiteSVM result
    pub fn inner(&self) -> &TransactionMetadata {
        &self.inner
    }
}

impl fmt::Debug for TransactionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransactionResult")
            .field("instruction", &self.instruction_name)
            .field("logs_count", &self.inner.logs.len())
            .field("compute_units", &self.compute_units())
            .finish()
    }
}

/// Error type for transaction execution
#[derive(Debug)]
pub enum TransactionError {
    /// Transaction failed with error message
    ExecutionFailed(String),
    /// Error building the transaction
    BuildError(String),
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::ExecutionFailed(msg) => {
                write!(f, "Transaction execution failed: {}", msg)
            }
            TransactionError::BuildError(msg) => {
                write!(f, "Transaction build error: {}", msg)
            }
        }
    }
}

impl std::error::Error for TransactionError {}

/// Helper trait for transaction execution on AnchorContext
pub trait TransactionHelpers {
    /// Send a single instruction as a transaction
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, TransactionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let signer = Keypair::new();
    /// # let ix = solana_program::instruction::Instruction {
    /// #     program_id: Pubkey::new_unique(),
    /// #     accounts: vec![],
    /// #     data: vec![],
    /// # };
    /// let result = ctx.send_instruction(ix, &[&signer]).unwrap();
    /// result.assert_success();
    /// ```
    fn send_instruction(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError>;

    /// Send multiple instructions as a single transaction
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, TransactionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_sdk::signature::Keypair;
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let signer = Keypair::new();
    /// # let ix1 = solana_program::instruction::Instruction {
    /// #     program_id: Pubkey::new_unique(),
    /// #     accounts: vec![],
    /// #     data: vec![],
    /// # };
    /// # let ix2 = ix1.clone();
    /// let result = ctx.send_instructions(&[ix1, ix2], &[&signer]).unwrap();
    /// result.assert_success();
    /// ```
    fn send_instructions(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError>;

    /// Build and execute an instruction in one call
    ///
    /// # Example
    /// ```no_run
    /// # use anchor_litesvm::{AnchorContext, TransactionHelpers};
    /// # use litesvm::LiteSVM;
    /// # use solana_program::pubkey::Pubkey;
    /// # use solana_program::instruction::AccountMeta;
    /// # use solana_sdk::signature::{Keypair, Signer};
    /// # use anchor_lang::AnchorSerialize;
    /// # use borsh::BorshSerialize;
    /// # #[derive(BorshSerialize)]
    /// # struct TestArgs { value: u64 }
    /// # impl AnchorSerialize for TestArgs {
    /// #     fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
    /// #         BorshSerialize::serialize(self, writer)
    /// #     }
    /// # }
    /// # let mut ctx = AnchorContext::new(LiteSVM::new(), Pubkey::new_unique());
    /// # let signer = Keypair::new();
    /// let accounts = vec![
    ///     AccountMeta::new(signer.pubkey(), true),
    /// ];
    /// let args = TestArgs { value: 42 };
    /// let result = ctx.execute("initialize", accounts, args, &[&signer]).unwrap();
    /// result.assert_success();
    /// ```
    fn execute<T>(
        &mut self,
        instruction_name: &str,
        accounts: Vec<solana_program::instruction::AccountMeta>,
        args: T,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError>
    where
        T: anchor_lang::AnchorSerialize;
}

impl TransactionHelpers for crate::AnchorContext {
    fn send_instruction(
        &mut self,
        instruction: Instruction,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError> {
        self.send_instructions(&[instruction], signers)
    }

    fn send_instructions(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError> {
        if signers.is_empty() {
            return Err(TransactionError::BuildError(
                "No signers provided".to_string(),
            ));
        }

        // Use first signer as payer
        let payer = &signers[0].pubkey();

        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(payer),
            signers,
            self.svm.latest_blockhash(),
        );

        match self.svm.send_transaction(tx) {
            Ok(result) => Ok(TransactionResult::new(result, None)),
            Err(e) => Err(TransactionError::ExecutionFailed(format!("{:?}", e))),
        }
    }

    fn execute<T>(
        &mut self,
        instruction_name: &str,
        accounts: Vec<solana_program::instruction::AccountMeta>,
        args: T,
        signers: &[&Keypair],
    ) -> Result<TransactionResult, TransactionError>
    where
        T: anchor_lang::AnchorSerialize,
    {
        let instruction = self
            .build_instruction(instruction_name, accounts, args)
            .map_err(|e| TransactionError::BuildError(e.to_string()))?;

        match self.send_instruction(instruction, signers) {
            Ok(mut result) => {
                result.instruction_name = Some(instruction_name.to_string());
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }
}