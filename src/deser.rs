use crate::account::Account;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

fn deserialize_opt_decimal_with_precision<'de, D>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt_decimal: Option<Decimal> = Option::deserialize(deserializer)?;
    Ok(opt_decimal.map(|val| val.round_dp(4))) // Bankers rounding
}

/// Represents a transaction record issued by a source (e.g. CSV file)
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Record {
    #[serde(rename = "type")]
    pub command: String,
    pub client: u16,
    pub tx: u32,
    #[serde(deserialize_with = "deserialize_opt_decimal_with_precision")]
    pub amount: Option<Decimal>,
}

/// This struct represent a CSV record for the output file
#[derive(Serialize)]
pub struct OutRecord {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl From<&Account> for OutRecord {
    fn from(value: &Account) -> Self {
        Self {
            client: value.id,
            available: value.available,
            held: value.held,
            total: value.total,
            locked: value.locked,
        }
    }
}
