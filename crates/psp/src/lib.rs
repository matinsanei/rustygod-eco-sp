//! Real payment service providers (Stripe today, Adyen-shaped tomorrow).

pub mod stripe;

pub use stripe::{StripePsp, STRIPE_API_BASE};
