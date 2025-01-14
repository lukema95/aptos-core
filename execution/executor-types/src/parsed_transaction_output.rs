// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    contract_event::ContractEvent,
    transaction::{
        Transaction, TransactionAuxiliaryData, TransactionOutput, TransactionOutputProvider,
        TransactionStatus,
    },
    write_set::WriteSet,
};
use itertools::zip_eq;
use std::ops::Deref;

#[derive(Clone)]
pub struct ParsedTransactionOutput {
    output: TransactionOutput,
    reconfig_events: Vec<ContractEvent>,
}

impl ParsedTransactionOutput {
    pub fn parse_reconfig_events(events: &[ContractEvent]) -> impl Iterator<Item = &ContractEvent> {
        events.iter().filter(|e| e.is_new_epoch_event())
    }
}

impl TransactionOutputProvider for ParsedTransactionOutput {
    fn get_transaction_output(&self) -> &TransactionOutput {
        &self.output
    }
}

impl From<TransactionOutput> for ParsedTransactionOutput {
    fn from(output: TransactionOutput) -> Self {
        let reconfig_events = Self::parse_reconfig_events(output.events())
            .cloned()
            .collect();
        Self {
            output,
            reconfig_events,
        }
    }
}

impl Deref for ParsedTransactionOutput {
    type Target = TransactionOutput;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl ParsedTransactionOutput {
    pub fn is_reconfig(&self) -> bool {
        !self.reconfig_events.is_empty()
    }

    pub fn unpack(
        self,
    ) -> (
        WriteSet,
        Vec<ContractEvent>,
        Vec<ContractEvent>,
        u64,
        TransactionStatus,
        TransactionAuxiliaryData,
    ) {
        let Self {
            output,
            reconfig_events,
        } = self;
        let (write_set, events, gas_used, status, auxiliary_data) = output.unpack();

        (
            write_set,
            events,
            reconfig_events,
            gas_used,
            status,
            auxiliary_data,
        )
    }
}

#[derive(Default)]
pub struct TransactionsWithParsedOutput {
    transactions: Vec<Transaction>,
    parsed_output: Vec<ParsedTransactionOutput>,
}

impl TransactionsWithParsedOutput {
    pub fn new(transaction: Vec<Transaction>, parsed_output: Vec<ParsedTransactionOutput>) -> Self {
        assert_eq!(
            transaction.len(),
            parsed_output.len(),
            "transaction.len(): {}, parsed_output.len(): {}",
            transaction.len(),
            parsed_output.len()
        );
        Self {
            transactions: transaction,
            parsed_output,
        }
    }

    pub fn new_empty() -> Self {
        Self::default()
    }

    pub fn push(&mut self, transaction: Transaction, parsed_output: ParsedTransactionOutput) {
        self.transactions.push(transaction);
        self.parsed_output.push(parsed_output);
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn txns(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    pub fn parsed_outputs(&self) -> &Vec<ParsedTransactionOutput> {
        &self.parsed_output
    }

    pub fn get_last_checkpoint_index(&self) -> Option<usize> {
        (0..self.len())
            .rev()
            .find(|&i| Self::need_checkpoint(&self.transactions[i], &self.parsed_output[i]))
    }

    pub fn need_checkpoint(txn: &Transaction, txn_output: &ParsedTransactionOutput) -> bool {
        if txn_output.is_reconfig() {
            return true;
        }
        match txn {
            Transaction::BlockMetadata(_)
            | Transaction::BlockMetadataExt(_)
            | Transaction::UserTransaction(_)
            | Transaction::ValidatorTransaction(_) => false,
            Transaction::GenesisTransaction(_)
            | Transaction::StateCheckpoint(_)
            | Transaction::BlockEpilogue(_) => true,
        }
    }

    pub fn into_txns(self) -> Vec<Transaction> {
        self.transactions
    }

    pub fn into_inner(self) -> (Vec<Transaction>, Vec<TransactionOutput>) {
        (
            self.transactions,
            self.parsed_output.into_iter().map(|t| t.output).collect(),
        )
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Transaction, &ParsedTransactionOutput)> {
        zip_eq(self.transactions.iter(), self.parsed_output.iter())
    }
}

impl From<Vec<(Transaction, ParsedTransactionOutput)>> for TransactionsWithParsedOutput {
    fn from(value: Vec<(Transaction, ParsedTransactionOutput)>) -> Self {
        let (transaction, parsed_output) = value.into_iter().unzip();
        Self::new(transaction, parsed_output)
    }
}
