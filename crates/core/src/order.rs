use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::checkout::Checkout;
use crate::money::Money;

/// Mirrors `saleor/order/__init__.py::OrderStatus` — exact Django strings,
/// including the `"partially fulfilled"` space (not underscore).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Draft,
    Unconfirmed,
    Unfulfilled,
    PartiallyFulfilled,
    Fulfilled,
    PartiallyReturned,
    Returned,
    Canceled,
    Expired,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Unconfirmed => "unconfirmed",
            Self::Unfulfilled => "unfulfilled",
            Self::PartiallyFulfilled => "partially fulfilled",
            Self::Fulfilled => "fulfilled",
            Self::PartiallyReturned => "partially_returned",
            Self::Returned => "returned",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "draft" => Self::Draft,
            "unconfirmed" => Self::Unconfirmed,
            "partially fulfilled" => Self::PartiallyFulfilled,
            "fulfilled" => Self::Fulfilled,
            "partially_returned" => Self::PartiallyReturned,
            "returned" => Self::Returned,
            "canceled" => Self::Canceled,
            "expired" => Self::Expired,
            _ => Self::Unfulfilled,
        }
    }
}

/// Mirrors `saleor/order/models.py`: OrderLine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLine {
    pub variant_id: String,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: Money,
    pub total_price: Money,
}

/// Mirrors `saleor/order/models.py`: Order.
/// Created from a Checkout — mirrors `saleor/checkout/complete_checkout.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub number: String,
    pub channel: String,
    pub email: String,
    pub status: OrderStatus,
    pub lines: Vec<OrderLine>,
    pub total: Money,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

impl Order {
    pub fn from_checkout(
        checkout: &Checkout,
        resolve_name: impl Fn(&str) -> String,
        next_number: impl Fn() -> String,
    ) -> Self {
        use rust_decimal::Decimal;
        let lines = checkout
            .lines
            .iter()
            .map(|l| OrderLine {
                variant_id: l.variant_id.clone(),
                product_name: resolve_name(&l.variant_id),
                quantity: l.quantity,
                unit_price: l.unit_price.clone(),
                total_price: Money::new(
                    l.unit_price.amount * Decimal::from(l.quantity),
                    l.unit_price.currency.clone(),
                ),
            })
            .collect();
        let total = checkout.total();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            number: next_number(),
            channel: checkout.channel.clone(),
            email: checkout.email.clone(),
            status: OrderStatus::Unfulfilled,
            lines,
            total: total.clone(),
            currency: total.currency,
            created_at: Utc::now(),
        }
    }

    pub fn to_proto(&self) -> rustygod_proto::order::Order {
        rustygod_proto::order::Order {
            id: self.id.clone(),
            number: self.number.clone(),
            channel: self.channel.clone(),
            email: self.email.clone(),
            status: self.status.as_str().to_string(),
            lines: self
                .lines
                .iter()
                .map(|l| rustygod_proto::order::OrderLine {
                    variant_id: l.variant_id.clone(),
                    product_name: l.product_name.clone(),
                    quantity: l.quantity,
                    unit_price: Some(l.unit_price.clone().into()),
                    total_price: Some(l.total_price.clone().into()),
                })
                .collect(),
            total: Some(self.total.clone().into()),
            currency: self.currency.clone(),
            created_at: self.created_at.to_rfc3339(),
        }
    }
}
