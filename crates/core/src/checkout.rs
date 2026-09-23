use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::money::Money;
use crate::{DomainError, Result};

/// Mirrors `saleor/checkout/models.py`: Checkout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkout {
    pub id: String,
    pub channel: String,
    pub email: String,
    pub currency: String,
    pub lines: Vec<CheckoutLine>,
}

/// Mirrors `saleor/checkout/models.py`: CheckoutLine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutLine {
    pub variant_id: String,
    pub quantity: i32,
    pub unit_price: Money,
    /// Free promotion gift: priced from the variant listing for display but
    /// contributes ZERO to totals (Django's gift line totals 0 via its
    /// covering line discount). Defaults false for back-compat payloads.
    #[serde(default)]
    pub is_gift: bool,
}

impl Checkout {
    pub fn new(channel: impl Into<String>, email: impl Into<String>, currency: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel: channel.into(),
            email: email.into(),
            currency: currency.to_string(),
            lines: Vec::new(),
        }
    }

    pub fn add_line(&mut self, variant_id: String, quantity: i32, unit_price: Money) -> Result<()> {
        if quantity <= 0 {
            return Err(DomainError::Validation {
                field: "quantity".into(),
                message: "quantity must be positive".into(),
            });
        }
        if unit_price.currency != self.currency {
            return Err(DomainError::Validation {
                field: "currency".into(),
                message: "line currency must match checkout currency".into(),
            });
        }
        match self.lines.iter_mut().find(|l| l.variant_id == variant_id) {
            Some(line) => line.quantity += quantity,
            None => self.lines.push(CheckoutLine {
                variant_id,
                quantity,
                unit_price,
                is_gift: false,
            }),
        }
        Ok(())
    }

    /// Payable total: gift lines contribute zero (their covering discount
    /// zeroes them in Django; same net effect here).
    pub fn total(&self) -> Money {
        let amount = self
            .lines
            .iter()
            .filter(|l| !l.is_gift)
            .map(|l| l.unit_price.amount * Decimal::from(l.quantity))
            .sum();
        Money::new(amount, self.currency.clone())
    }

    pub fn to_proto(&self) -> saleor_rustify_proto::checkout::Checkout {
        let total = self.total();
        saleor_rustify_proto::checkout::Checkout {
            id: self.id.clone(),
            channel: self.channel.clone(),
            email: self.email.clone(),
            lines: self
                .lines
                .iter()
                .map(|l| saleor_rustify_proto::checkout::CheckoutLine {
                    variant_id: l.variant_id.clone(),
                    quantity: l.quantity,
                    unit_price: Some(l.unit_price.clone().into()),
                    total_price: Some(
                        Money::new(
                            if l.is_gift {
                                Decimal::ZERO
                            } else {
                                l.unit_price.amount * Decimal::from(l.quantity)
                            },
                            l.unit_price.currency.clone(),
                        )
                        .into(),
                    ),
                    is_gift: l.is_gift,
                })
                .collect(),
            total: Some(total.into()),
            currency: self.currency.clone(),
        }
    }
}
