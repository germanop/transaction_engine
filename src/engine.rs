use crate::account::{Account, Operation};
use crate::deser::Record;
use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

/// This is the Transaction Engine struct.
///
/// This object contains all the transactions logic and can be run in its own thread.
pub struct Engine {
    accounts: HashMap<u16, Account>,
    tx_record: HashMap<u32, (u16, Decimal)>, // it seems only deposits can be disputed, so we just need amount and client_id
    dispute_record: HashSet<u32>,            // Check if a transaction is under dispute
}

impl Engine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            tx_record: HashMap::new(),
            dispute_record: HashSet::new(),
        }
    }

    /// Executes instructions contained in a Record (command)
    pub fn process(&mut self, record: &Record) -> Result<()> {
        match record.command.as_str() {
            "deposit" => {
                let amount = record.amount.ok_or_else(|| anyhow!("Missing amount"))?;
                let account = self.get_account(record.client);
                account.execute(Operation::Deposit, amount)?;
                self.register_transaction(record.tx, record.client, amount);
            }
            "withdrawal" => {
                let amount = record.amount.ok_or_else(|| anyhow!("Missing amount"))?;
                let account = self.get_account(record.client);
                account.execute(Operation::Withdraw, amount)?;
                // We do not record withdrawals
            }
            "dispute" => {
                // Check if the transaction has not been disputed already
                if self.dispute_record.contains(&record.tx) {
                    return Err(anyhow!("Transaction already under dispute"));
                }
                // Check transaction exists and belongs to the right client
                let (client_id, amount) = *self
                    .tx_record
                    .get(&record.tx)
                    .ok_or_else(|| anyhow!("Transaction not found"))?;
                if client_id != record.client {
                    return Err(anyhow!("Transaction does not belong to client"));
                }

                let account = self.get_account(record.client);
                account.execute(Operation::Dispute, amount)?;
                self.dispute_record.insert(record.tx);
            }
            "resolve" => {
                // Check if tx under dispute
                if !self.dispute_record.contains(&record.tx) {
                    return Err(anyhow!("Transaction not under dispute"));
                }
                // Get transaction details, if any, and if the client is the correct one
                let (client_id, amount) = *self
                    .tx_record
                    .get(&record.tx)
                    .ok_or_else(|| anyhow!("Transaction not found"))?;
                if client_id != record.client {
                    return Err(anyhow!("Transaction does not belong to client"));
                }

                let account = self.get_account(record.client);
                account.execute(Operation::Resolve, amount)?;
                self.dispute_record.remove(&record.tx);
            }
            "chargeback" => {
                // Check if tx under dispute
                if !self.dispute_record.contains(&record.tx) {
                    return Err(anyhow!("Transaction not under dispute"));
                }
                // Get transaction details, if any, and if the client is the correct one
                let (client_id, amount) = *self
                    .tx_record
                    .get(&record.tx)
                    .ok_or_else(|| anyhow!("Transaction not found"))?;
                if client_id != record.client {
                    return Err(anyhow!("Transaction does not belong to client"));
                }

                let account = self.get_account(record.client);
                account.execute(Operation::Chargeback, amount)?;
                self.dispute_record.remove(&record.tx);
            }
            _ => {
                return Err(anyhow!("Unknown command"));
            }
        }
        Ok(())
    }

    /// Retrieve Account given its id. Create one if it does not exist
    fn get_account(&mut self, account_id: u16) -> &mut Account {
        self.accounts
            .entry(account_id)
            .or_insert(Account::new(account_id))
    }

    /// Register transaction in our internal hashmap
    fn register_transaction(&mut self, tx: u32, client_id: u16, amount: Decimal) {
        self.tx_record.insert(tx, (client_id, amount)); // tx are supposed to be unique, so insert is never updating
    }

    /// This is just a placeholder for code running `Engine` as a standalone service.
    ///
    /// We are not using this: how this is run is defined in the main thread.
    #[allow(dead_code)]
    pub fn run(&mut self) {
        unimplemented!()
    }

    /// Utility function returning all the known accounts.
    ///
    /// The idea is to use the returned value to print accounts out in a format of user's choosing.
    pub fn get_accounts(&self) -> &HashMap<u16, Account> {
        &self.accounts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_ok() {
        let mut engine = Engine::new();
        let record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&record).unwrap();
        assert_eq!(engine.tx_record.len(), 1);
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    #[test]
    fn test_deposit_no_amount() {
        let mut engine = Engine::new();
        let record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: None,
            tx: 1,
        };
        assert!(engine.process(&record).is_err());
        assert_eq!(engine.tx_record.len(), 0);
        assert_eq!(engine.accounts.len(), 0);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    #[test]
    fn test_withdrawal_ok() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 2,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        let record = Record {
            client: 2,
            command: "withdrawal".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 3,
        };
        engine.process(&deposit_record).unwrap();
        engine.process(&record).unwrap();
        assert_eq!(engine.tx_record.len(), 1); // it's the deposit
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    #[test]
    fn test_withdrawal_no_amount() {
        let mut engine = Engine::new();
        let record = Record {
            client: 1,
            command: "withdrawal".to_string(),
            amount: None,
            tx: 1,
        };
        assert!(engine.process(&record).is_err());
        assert_eq!(engine.tx_record.len(), 0);
        assert_eq!(engine.accounts.len(), 0);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    #[test]
    fn test_dispute_ok() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&deposit_record).unwrap();

        let record = Record {
            client: 1,
            command: "dispute".to_string(),
            amount: None,
            tx: 1,
        };
        engine.process(&record).unwrap();
        assert_eq!(engine.tx_record.len(), 1);
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.dispute_record.len(), 1);
    }

    #[test]
    fn test_dispute_no_entry() {
        let mut engine = Engine::new();
        let record = Record {
            client: 1,
            command: "dispute".to_string(),
            amount: None,
            tx: 1,
        };
        assert!(engine.process(&record).is_err());
        assert_eq!(engine.tx_record.len(), 0);
        assert_eq!(engine.accounts.len(), 0);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    #[test]
    fn test_dispute_wrong_client() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&deposit_record).unwrap();

        let record = Record {
            client: 2,
            command: "dispute".to_string(),
            amount: None,
            tx: 1,
        };
        assert!(engine.process(&record).is_err());
        assert_eq!(engine.tx_record.len(), 1);
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    #[test]
    fn test_resolve_ok() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&deposit_record).unwrap();

        // Now dispute it
        let dispute_record = Record {
            client: 1,
            command: "dispute".to_string(),
            amount: None,
            tx: 1,
        };
        engine.process(&dispute_record).unwrap();
        assert_eq!(engine.dispute_record.len(), 1);

        // Now resolve it
        let record = Record {
            client: 1,
            command: "resolve".to_string(),
            amount: None,
            tx: 1,
        };
        engine.process(&record).unwrap();
        assert_eq!(engine.tx_record.len(), 1);
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    #[test]
    fn test_resolve_undisputed() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&deposit_record).unwrap();

        // Now resolve it
        let record = Record {
            client: 1,
            command: "resolve".to_string(),
            amount: None,
            tx: 1,
        };
        assert!(engine.process(&record).is_err());
        assert_eq!(engine.tx_record.len(), 1);
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    // ToDo: test_resolve_wrong_client

    #[test]
    fn test_chargeback_ok() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&deposit_record).unwrap();

        // Now dispute it
        let dispute_record = Record {
            client: 1,
            command: "dispute".to_string(),
            amount: None,
            tx: 1,
        };
        engine.process(&dispute_record).unwrap();
        assert_eq!(engine.dispute_record.len(), 1);

        // Now chargeback
        let record = Record {
            client: 1,
            command: "chargeback".to_string(),
            amount: None,
            tx: 1,
        };
        engine.process(&record).unwrap();
        assert_eq!(engine.tx_record.len(), 1);
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    #[test]
    fn test_chargeback_undisputed() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&deposit_record).unwrap();

        // Now chargeback
        let record = Record {
            client: 1,
            command: "chargeback".to_string(),
            amount: None,
            tx: 1,
        };
        assert!(engine.process(&record).is_err());
        assert_eq!(engine.tx_record.len(), 1);
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.dispute_record.len(), 0);
    }

    // ToDo: test_chargeback_wrong_client

    #[test]
    fn test_get_accounts() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&deposit_record).unwrap();

        let deposit_record = Record {
            client: 2,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 2,
        };
        engine.process(&deposit_record).unwrap();

        assert_eq!(engine.get_accounts().len(), 2);
    }

    #[test]
    fn test_wrong_command() {
        let mut engine = Engine::new();
        let deposit_record = Record {
            client: 1,
            command: "deposit".to_string(),
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        engine.process(&deposit_record).unwrap();

        let record = Record {
            client: 1,
            command: "withdraw".to_string(), // it's withdrawal
            amount: Some(Decimal::new(100, 1)),
            tx: 1,
        };
        assert!(engine.process(&record).is_err());
        assert_eq!(engine.accounts.len(), 1);
        assert_eq!(engine.tx_record.len(), 1);
        assert_eq!(engine.dispute_record.len(), 0);
    }
}
