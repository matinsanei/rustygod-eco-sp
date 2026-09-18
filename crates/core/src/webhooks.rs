//! Webhook signing + retry schedule, mirroring
//! `saleor/webhook/transport/__init__.py::signature_for_payload`,
//! `saleor/app/headers.py`, and the celery retry policy
//! (`countdown = backoff * 2^retries`, delivery max 12 retries).

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// `signature_for_payload` with a secret: HMAC-SHA256 hex of the body.
/// (Without a secret Django falls back to JWS; Rust signs HMAC-only and
/// requires webhooks to carry a secret — enforced at trigger time.)
pub fn sign_payload(body: &[u8], secret_key: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(body);
    hex_encode(&mac.finalize().into_bytes())
}

/// Constant-time verification (receivers use this; Django compares with
/// `hmac.compare_digest` server-side equivalents).
pub fn verify_signature(body: &[u8], secret_key: &str, signature: &str) -> bool {
    let expected = sign_payload(body, secret_key);
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

/// Saleor request headers for a delivery.
pub fn delivery_headers(domain: &str, event_type: &str, signature: &str) -> Vec<(String, String)> {
    vec![
        ("Saleor-Domain".into(), domain.into()),
        ("Saleor-Event".into(), event_type.into()),
        ("Saleor-Signature".into(), signature.into()),
        ("Content-Type".into(), "application/json".into()),
    ]
}

/// Celery-style retry countdown: `backoff * 2^retries` seconds.
pub fn retry_countdown_secs(backoff_secs: u64, retries: u32) -> u64 {
    backoff_secs.saturating_mul(2u64.saturating_pow(retries.min(20)))
}

/// Delivery attempts stop after this many tries (Django: max_retries=12).
pub const MAX_DELIVERY_RETRIES: u32 = 12;
/// Base backoff between attempts, seconds (Django: retry_backoff=10).
pub const DELIVERY_BACKOFF_SECS: u64 = 10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_reference_vector() {
        // Cross-checked against Python:
        // hmac.new(b"secret", b"{}", hashlib.sha256).hexdigest()
        assert_eq!(
            sign_payload(b"{}", "secret"),
            "77325902caca812dc259733aacd046b73817372c777b8d95b402647474516e13"
        );
    }

    #[test]
    fn verify_roundtrip_and_tamper() {
        let sig = sign_payload(b"{\"a\":1}", "s3cr3t");
        assert!(verify_signature(b"{\"a\":1}", "s3cr3t", &sig));
        assert!(!verify_signature(b"{\"a\":2}", "s3cr3t", &sig));
        assert!(!verify_signature(b"{\"a\":1}", "other", &sig));
    }

    #[test]
    fn retry_schedule_doubles() {
        assert_eq!(retry_countdown_secs(10, 0), 10);
        assert_eq!(retry_countdown_secs(10, 1), 20);
        assert_eq!(retry_countdown_secs(10, 3), 80);
    }
}
