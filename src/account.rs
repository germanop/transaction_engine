use anyhow::{anyhow, Result};
use rust_decimal::Decimal;

pub enum Operation {
    Deposit,
    Withdraw,
    Dispute,
    Resolve,
    Chargeback,
}

/// Client's account
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Account {
    pub id: u16,
    pub locked: bool,
    pub total: Decimal,
    pub available: Decimal,
    pub held: Decimal,
}

impl Account {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            locked: false,
            total: Decimal::ZERO,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
        }
    }

    /// This is the main interface for account operations. Most of the checks are run here.
    ///
    /// This function runs the underlying operations only if Account is not locked and `amount`
    /// is non-negative.
    pub fn execute(&mut self, operation: Operation, amount: Decimal) -> Result<()> {
        if self.locked {
            return Err(anyhow!("Account is locked"));
        }

        if amount.is_sign_negative() {
            return Err(anyhow!("Amount must be non-negative"));
        }

        match operation {
            Operation::Deposit => self.deposit(amount),
            Operation::Withdraw => self.withdraw(amount),
            Operation::Dispute => self.dispute(amount),
            Operation::Resolve => self.resolve(amount),
            Operation::Chargeback => self.chargeback(amount),
        }
    }

    /// Add `amount` to client's balance.
    ///
    /// Total and available funds will increase.
    /// This function returns an error if `amount` will make it overflow
    ///
    /// # Warning
    /// This function should be used through the `execute` interface only.
    fn deposit(&mut self, amount: Decimal) -> Result<()> {
        // Add but beware of overflows
        self.total = self.total.checked_add(amount).ok_or(anyhow!("Overflow"))?;
        self.available = self
            .available
            .checked_add(amount)
            .ok_or(anyhow!("Overflow"))?; // If total did not overflow, neither should this
        Ok(())
    }

    /// Subtract `amount` to client's balance.
    ///
    /// Total and available funds will decrease.
    /// This function returns an error if `amount` is greater than available funds.
    /// It does not overflow.
    ///
    /// # Warning
    /// This function should be used through the `execute` interface only.
    fn withdraw(&mut self, amount: Decimal) -> Result<()> {
        // Are there enough funds?
        if amount > self.available {
            return Err(anyhow!("Insufficient funds"));
        }

        // By design, this can never overflow: fields are always ensured to be non-negative, and
        // we already checked `amount` is not bigger than `available`. It's safe to use `-=`
        self.total -= amount;
        self.available -= amount;
        Ok(())
    }

    /// Dispute a (deposit) transaction
    ///
    /// Held funds will increase by the amount specified, and available will decrease, so total will stay the same.
    /// This function returns an error if `amount` is greater than available funds.
    ///
    /// # Warning
    /// This function should be used through the `execute` interface only.
    ///
    /// # Note
    /// My understanding from the assignment text is that the only things you can dispute are deposits.
    /// It's an error to dispute more than available is also another assumption of mine. See README
    fn dispute(&mut self, amount: Decimal) -> Result<()> {
        // Are there enough funds?
        if amount > self.available {
            return Err(anyhow!("Insufficient funds"));
        }

        // I am not checking for overflows. The assumption is that `held` cannot get greater than available
        self.held += amount;
        self.available -= amount;

        Ok(())
    }

    /// Resolve a (deposit) transaction
    ///
    /// This function does reverse `dispute`.
    /// This function is not supposed to return an error (but it could if there is a flaw in the caller code).
    /// It does not overflow.
    ///
    /// # Warning
    /// This function should be used through the `execute` interface only.
    fn resolve(&mut self, amount: Decimal) -> Result<()> {
        // Are there enough held funds?
        if amount > self.held {
            return Err(anyhow!("Insufficient held funds"));
        }

        // This cannot overflow, because `available` cannot get greater than `total`.
        self.available += amount;
        self.held -= amount;

        Ok(())
    }

    /// Reverse (deposit) transaction's `amount` and lock it.
    ///
    /// Total and held funds will decrease.
    /// This function returns an error if `amount` is greater than held funds.
    /// It does not overflow.
    ///
    /// # Warning
    /// This function should be used through the `execute` interface only.
    fn chargeback(&mut self, amount: Decimal) -> Result<()> {
        // Are there enough held funds?
        if amount > self.held {
            return Err(anyhow!("Insufficient held funds"));
        }

        // By design, this can never overflow: fields are always ensured to be non-negative, and
        // we already checked `amount` is not bigger than `available`. It's safe to use `-=`
        self.total -= amount;
        self.held -= amount;
        self.locked = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // If account is locked is checked only through the `execute` interface
    #[test]
    fn test_account_locked() {
        let mut account = Account::new(1);
        account.locked = true;
        assert!(account.execute(Operation::Deposit, Decimal::ONE).is_err());
        assert_eq!(account.total, Decimal::ZERO);
        assert_eq!(account.available, Decimal::ZERO);
        assert_eq!(account.held, Decimal::ZERO);
        // now unlock it
        account.locked = false;
        assert!(account.execute(Operation::Deposit, Decimal::ONE).is_ok());
        // now lock and check balances are untouched
        account.locked = true;
        assert!(account.execute(Operation::Withdraw, Decimal::ONE).is_err());
        assert_eq!(account.total, Decimal::ONE);
        assert_eq!(account.available, Decimal::ONE);
        assert_eq!(account.held, Decimal::ZERO);
    }

    // If `amount` is negative is checked only through the `execute` interface
    #[test]
    fn test_negative_amount() {
        let mut account = Account::new(1);
        assert!(account
            .execute(Operation::Deposit, Decimal::new(-1, 0))
            .is_err());
        assert_eq!(account.total, Decimal::ZERO);
        assert_eq!(account.available, Decimal::ZERO);
    }

    #[test]
    fn test_deposit_ok() {
        let mut account = Account::new(1);
        account.deposit(Decimal::ONE).unwrap();
        assert_eq!(account.total, Decimal::ONE);
        assert_eq!(account.available, Decimal::ONE);
        assert_eq!(account.held, Decimal::ZERO);
    }

    #[test]
    fn test_deposit_overflow() {
        let mut account = Account::new(1);
        account.deposit(Decimal::ONE).unwrap();
        assert!(account.deposit(Decimal::MAX).is_err());
        // Check balances are unaffected
        assert_eq!(account.total, Decimal::ONE);
        assert_eq!(account.available, Decimal::ONE);
        assert_eq!(account.held, Decimal::ZERO);
    }

    #[test]
    fn test_withdraw_ok() {
        let mut account = Account {
            id: 1,
            locked: false,
            total: Decimal::TWO,
            available: Decimal::TWO,
            held: Decimal::ZERO,
        };
        account.withdraw(Decimal::ONE).unwrap();
        assert_eq!(account.total, Decimal::ONE);
        assert_eq!(account.available, Decimal::ONE);
        assert_eq!(account.held, Decimal::ZERO);
    }

    #[test]
    fn test_withdraw_insufficient_funds() {
        let mut account = Account {
            id: 1,
            locked: false,
            total: Decimal::ONE,
            available: Decimal::ONE,
            held: Decimal::ZERO,
        };
        assert!(account.withdraw(Decimal::TWO).is_err());
        // Check balances are unaffected
        assert_eq!(account.total, Decimal::ONE);
        assert_eq!(account.available, Decimal::ONE);
        assert_eq!(account.held, Decimal::ZERO);
    }

    #[test]
    fn test_dispute_ok() {
        let mut account = Account::new(1);
        account.deposit(Decimal::TWO).unwrap();
        account.dispute(Decimal::ONE).unwrap();
        assert_eq!(account.total, Decimal::TWO);
        assert_eq!(account.available, Decimal::ONE);
        assert_eq!(account.held, Decimal::ONE);
    }

    #[test]
    fn test_dispute_too_big() {
        let mut account = Account {
            id: 1,
            locked: false,
            total: Decimal::TWO,
            available: Decimal::ONE,
            held: Decimal::ONE,
        };
        let expected = account.clone();
        assert!(account.dispute(Decimal::TWO).is_err());
        assert_eq!(account, expected);
    }

    #[test]
    fn test_resolve_ok() {
        let mut account = Account {
            id: 1,
            locked: false,
            total: Decimal::TWO,
            available: Decimal::ONE,
            held: Decimal::ONE,
        };
        account.resolve(Decimal::ONE).unwrap();
        let expected = Account {
            id: 1,
            locked: false,
            total: Decimal::TWO,
            available: Decimal::TWO,
            held: Decimal::ZERO,
        };
        assert_eq!(account, expected);
    }

    #[test]
    fn test_resolve_insufficient_funds() {
        let mut account = Account {
            id: 1,
            locked: false,
            total: Decimal::TWO,
            available: Decimal::ONE,
            held: Decimal::ONE,
        };
        let expected = account.clone();
        assert!(account.resolve(Decimal::TWO).is_err());
        assert_eq!(account, expected);
    }

    #[test]
    fn test_chargeback_ok() {
        let mut account = Account {
            id: 1,
            locked: false,
            total: Decimal::TWO,
            available: Decimal::ONE,
            held: Decimal::ONE,
        };
        account.chargeback(Decimal::ONE).unwrap();
        let expected = Account {
            id: 1,
            locked: true,
            total: Decimal::ONE,
            available: Decimal::ONE,
            held: Decimal::ZERO,
        };
        assert_eq!(account, expected);
    }
}
