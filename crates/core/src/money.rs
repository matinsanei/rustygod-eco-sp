use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Mirrors saleor's `Money` (prices.Money): decimal amount + ISO currency.
/// Amount is serialized as string on the wire (see common.proto) — never float.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount: Decimal,
    pub currency: String,
}

impl Money {
    pub fn new(amount: Decimal, currency: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into(),
        }
    }

    pub fn zero(currency: impl Into<String>) -> Self {
        Self {
            amount: Decimal::ZERO,
            currency: currency.into(),
        }
    }
}

impl From<Money> for saleor_rustify_proto::common::Money {
    fn from(m: Money) -> Self {
        Self {
            currency: m.currency,
            amount: m.amount.to_string(),
        }
    }
}

impl TryFrom<saleor_rustify_proto::common::Money> for Money {
    type Error = crate::DomainError;

    fn try_from(m: saleor_rustify_proto::common::Money) -> crate::Result<Self> {
        let amount =
            m.amount
                .parse::<Decimal>()
                .map_err(|_| crate::DomainError::Validation {
                    field: "amount".into(),
                    message: "invalid decimal amount".into(),
                })?;
        Ok(Self {
            amount,
            currency: m.currency,
        })
    }
}
