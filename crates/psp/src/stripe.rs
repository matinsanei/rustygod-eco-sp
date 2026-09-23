//! Real PSP: Stripe PaymentIntents over HTTPS.
//!
//! The engine (`db::payments`) only speaks [`saleor_rustify_core::psp`] outcomes;
//! this crate translates Stripe's PaymentIntent/Refund/Charge API into them:
//! - `succeeded` → `Completed` (funds moved, single `*_success` downstream);
//! - `requires_action` + `redirect_to_url` → `ActionRequired` (3DS loop via
//!   `PspCallback`, same as the challenge sim but with a real redirect);
//! - `processing` / `requires_capture` (manual) → `Pending` (async settle);
//! - Stripe `error` / transport failure → `Failed` (nothing moves).
//!
//! Amounts go over the wire in minor units (`amount * 10^exp`) with a
//! zero-decimal currency table mirroring Stripe's docs. Every request sets
//! Stripe's `Idempotency-Key` from the engine key, so retries replay —
//! the PSP-dedup contract (T1b) holds end to end.
//!
//! Auth: `STRIPE_SECRET_KEY` (`sk_test_...`); `STRIPE_API_BASE` overrides
//! the host (tests point it at a local mock). No key → the server rejects
//! the `stripe` gateway instead of failing obscurely mid-flow.

use rust_decimal::Decimal;
use saleor_rustify_core::psp::{Psp, PspAction, PspOutcome, PspRequest};
use serde::Deserialize;

pub const STRIPE_API_BASE: &str = "https://api.stripe.com";

/// Zero-decimal currencies (Stripe bills these without minor units).
/// Full list: Stripe docs "zero-decimal currencies".
fn minor_exp(currency: &str) -> u32 {
    match currency.to_uppercase().as_str() {
        "BIF" | "CLP" | "DJF" | "GNF" | "JPY" | "KMF" | "KRW" | "MGA" | "PYG" | "RWF" | "UGX"
        | "UYU" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        // Stripe has no 3-decimal currencies in practice (BHD/JOD/KWD/OMD/TND
        // are 3-decimal at the network but Stripe uses 2); keep 2 default.
        _ => 2,
    }
}

fn minor_units(amount: Decimal, currency: &str) -> Result<i64, String> {
    use rust_decimal::RoundingStrategy;
    // Midpoint-away-from-zero: money rounds half-up, never banker's.
    let scaled = (amount * Decimal::from(10i64.pow(minor_exp(currency))))
        .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
    let divisor = 10i128.pow(scaled.scale());
    i64::try_from(scaled.mantissa() / divisor).map_err(|_| format!("amount {amount} out of range for {currency}"))
}

#[derive(Debug, Deserialize)]
struct StripeErrorBody {
    error: StripeError,
}

#[derive(Debug, Deserialize)]
struct StripeError {
    #[serde(default)]
    message: Option<String>,
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PaymentIntent {
    id: String,
    status: String,
    #[serde(default)]
    next_action: Option<NextAction>,
}

#[derive(Debug, Deserialize)]
struct NextAction {
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    redirect_to_url: Option<RedirectToUrl>,
}

#[derive(Debug, Deserialize)]
struct RedirectToUrl {
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Refund {
    id: String,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StripePsp {
    secret_key: String,
    api_base: String,
    client: reqwest::Client,
}

impl StripePsp {
    pub fn new(secret_key: impl Into<String>) -> Self {
        Self::with_base(secret_key, STRIPE_API_BASE)
    }

    pub fn with_base(secret_key: impl Into<String>, api_base: impl Into<String>) -> Self {
        Self {
            secret_key: secret_key.into(),
            api_base: api_base.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Build from the environment (`STRIPE_SECRET_KEY` required,
    /// `STRIPE_API_BASE` optional). None when unconfigured — the server
    /// turns that into a clean REJECTED instead of a transport failure.
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("STRIPE_SECRET_KEY").ok().filter(|k| !k.trim().is_empty())?;
        let base =
            std::env::var("STRIPE_API_BASE").ok().filter(|b| !b.trim().is_empty()).unwrap_or_else(|| STRIPE_API_BASE.into());
        Some(Self::with_base(key, base))
    }

    fn data_field(req: &PspRequest, key: &str) -> Option<String> {
        serde_json::from_str::<serde_json::Value>(req.data.as_deref()?).ok()?
            .get(key)?
            .as_str()
            .map(|s| s.to_string())
    }

    async fn post(&self, path: &str, form: &[(&str, String)], idempotency_key: &str) -> Result<reqwest::Response, String> {
        self.client
            .post(format!("{}{path}", self.api_base))
            .bearer_auth(&self.secret_key)
            .header("Idempotency-Key", idempotency_key)
            .header("Stripe-Version", "2024-06-20")
            .form(form)
            .send()
            .await
            .map_err(|e| format!("stripe transport: {e}"))
    }

    async fn parse_intent(&self, resp: reqwest::Response) -> PspOutcome {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return PspOutcome::Failed { error: stripe_message(&body).unwrap_or_else(|| format!("stripe HTTP {status}")) };
        }
        let pi: PaymentIntent = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(e) => return PspOutcome::Failed { error: format!("stripe bad intent body: {e}") },
        };
        intent_outcome(pi)
    }

    pub async fn execute_async(&self, req: &PspRequest) -> PspOutcome {
        match req.action {
            PspAction::Authorize => self.authorize(req).await,
            PspAction::Charge => self.charge(req).await,
            PspAction::Refund => self.refund(req).await,
            PspAction::Cancel => self.cancel(req).await,
        }
    }

    /// Manual-capture intent (`capture_method=manual`): funds held, moved on
    /// `charge` (capture). `data.payment_method` (`pm_...`, from Stripe.js)
    /// required; without it Stripe can't confirm → clean Failed, not panic.
    async fn authorize(&self, req: &PspRequest) -> PspOutcome {
        let amount = match minor_units(req.amount, &req.currency) {
            Ok(a) => a,
            Err(e) => return PspOutcome::Failed { error: e },
        };
        let Some(pm) = Self::data_field(req, "payment_method") else {
            return PspOutcome::Failed {
                error: "stripe authorize needs data.payment_method (pm_... from Stripe.js)".into(),
            };
        };
        let mut form: Vec<(&str, String)> = vec![
            ("amount", amount.to_string()),
            ("currency", req.currency.to_lowercase()),
            ("payment_method", pm),
            ("capture_method", "manual".into()),
            ("confirm", "true".into()),
            ("confirmation_method", "automatic".into()),
        ];
        if let Some(ret) = req.return_url.as_deref() {
            form.push(("return_url", ret.to_string()));
            form.push(("automatic_payment_methods[enabled]", "true".into()));
            form.push(("automatic_payment_methods[allow_redirects]", "always".into()));
        }
        match self.post("/v1/payment_intents", &form, &req.idempotency_key).await {
            Ok(r) => self.parse_intent(r).await,
            Err(e) => PspOutcome::Failed { error: e },
        }
    }

    /// Charge: capture a held intent (`data.payment_intent`) or — when none
    /// is given — create-and-capture in one step (automatic capture).
    async fn charge(&self, req: &PspRequest) -> PspOutcome {
        if let Some(pi) = Self::data_field(req, "payment_intent") {
            let amount = match minor_units(req.amount, &req.currency) {
                Ok(a) => a,
                Err(e) => return PspOutcome::Failed { error: e },
            };
            let form = [("amount_to_capture", amount.to_string())];
            match self.post(&format!("/v1/payment_intents/{pi}/capture"), &form, &req.idempotency_key).await {
                Ok(r) => self.parse_intent(r).await,
                Err(e) => PspOutcome::Failed { error: e },
            }
        } else {
            // One-step charge: same as authorize but automatic capture.
            let amount = match minor_units(req.amount, &req.currency) {
                Ok(a) => a,
                Err(e) => return PspOutcome::Failed { error: e },
            };
            let Some(pm) = Self::data_field(req, "payment_method") else {
                return PspOutcome::Failed {
                    error: "stripe charge needs data.payment_method or data.payment_intent".into(),
                };
            };
            let mut form: Vec<(&str, String)> = vec![
                ("amount", amount.to_string()),
                ("currency", req.currency.to_lowercase()),
                ("payment_method", pm),
                ("confirm", "true".into()),
            ];
            if let Some(ret) = req.return_url.as_deref() {
                form.push(("return_url", ret.to_string()));
            }
            match self.post("/v1/payment_intents", &form, &req.idempotency_key).await {
                Ok(r) => self.parse_intent(r).await,
                Err(e) => PspOutcome::Failed { error: e },
            }
        }
    }

    /// Refund a charge/intent. `data.charge` (`ch_...`) or
    /// `data.payment_intent` (`pi_...`, Stripe refunds the latest charge).
    async fn refund(&self, req: &PspRequest) -> PspOutcome {
        let amount = match minor_units(req.amount, &req.currency) {
            Ok(a) => a,
            Err(e) => return PspOutcome::Failed { error: e },
        };
        let mut form: Vec<(&str, String)> = vec![("amount", amount.to_string())];
        if let Some(ch) = Self::data_field(req, "charge") {
            form.push(("charge", ch));
        } else if let Some(pi) = Self::data_field(req, "payment_intent") {
            form.push(("payment_intent", pi));
        } else {
            return PspOutcome::Failed {
                error: "stripe refund needs data.charge or data.payment_intent".into(),
            };
        }
        let resp = match self.post("/v1/refunds", &form, &req.idempotency_key).await {
            Ok(r) => r,
            Err(e) => return PspOutcome::Failed { error: e },
        };
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return PspOutcome::Failed { error: stripe_message(&body).unwrap_or_else(|| format!("stripe HTTP {status}")) };
        }
        match serde_json::from_str::<Refund>(&body) {
            Ok(r) => match r.status.as_deref() {
                Some("succeeded") => PspOutcome::Completed { psp_reference: r.id },
                Some("pending") | Some("requires_action") | None => PspOutcome::Pending { psp_reference: r.id },
                Some("failed") | Some("canceled") => PspOutcome::Failed { error: format!("stripe refund {}", r.status.unwrap_or_default()) },
                Some(other) => PspOutcome::Pending { psp_reference: format!("{}:{other}", r.id) },
            },
            Err(e) => PspOutcome::Failed { error: format!("stripe bad refund body: {e}") },
        }
    }

    /// Cancel a held (uncaptured) intent. `data.payment_intent` required.
    async fn cancel(&self, req: &PspRequest) -> PspOutcome {
        let Some(pi) = Self::data_field(req, "payment_intent") else {
            return PspOutcome::Failed {
                error: "stripe cancel needs data.payment_intent of a held intent".into(),
            };
        };
        match self.post(&format!("/v1/payment_intents/{pi}/cancel"), &[], &req.idempotency_key).await {
            Ok(r) => self.parse_intent(r).await,
            Err(e) => PspOutcome::Failed { error: e },
        }
    }
}

fn stripe_message(body: &str) -> Option<String> {
    let e: StripeErrorBody = serde_json::from_str(body).ok()?;
    let mut m = e.error.message.unwrap_or_else(|| "stripe error".into());
    if let Some(code) = e.error.code {
        m.push_str(&format!(" ({code})"));
    } else if let Some(kind) = e.error.kind {
        m.push_str(&format!(" ({kind})"));
    }
    Some(m)
}

fn intent_outcome(pi: PaymentIntent) -> PspOutcome {
    match pi.status.as_str() {
        "succeeded" => PspOutcome::Completed { psp_reference: pi.id },
        "requires_capture" => PspOutcome::Pending { psp_reference: pi.id },
        "processing" => PspOutcome::Pending { psp_reference: pi.id },
        "requires_action" => match pi.next_action.as_ref().and_then(|n| n.redirect_to_url.as_ref()).and_then(|r| r.url.clone()) {
            Some(url) => PspOutcome::ActionRequired {
                psp_reference: pi.id,
                redirect_url: url,
                message: "Stripe 3DS challenge required".into(),
            },
            // Non-redirect challenges (webauthn etc.) can't run headless —
            // surface as pending for the webhook to settle.
            None => PspOutcome::Pending { psp_reference: pi.id },
        },
        "requires_payment_method" | "requires_confirmation" | "canceled" => {
            PspOutcome::Failed { error: format!("stripe intent {}: {}", pi.id, pi.status) }
        }
        other => PspOutcome::Failed { error: format!("stripe intent {}: unexpected status {other}", pi.id) },
    }
}

impl Psp for StripePsp {
    fn name(&self) -> &'static str {
        "stripe"
    }

    /// Sync bridge for the engine: blocks the calling thread on the async
    /// HTTP call. Only valid on a multi-thread tokio runtime (the server
    /// and all `#[tokio::test]`s); panics outside one — by design, a real
    /// PSP is a network call and must never run on a bare thread.
    fn execute(&self, req: &PspRequest) -> PspOutcome {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(self.execute_async(req)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saleor_rustify_core::psp::PspRequest;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct Seen {
        method_path: String,
        idempotency_key: String,
        body: String,
    }

    /// Minimal mock Stripe: routes POST paths to canned JSON, records the
    /// last request (path + Idempotency-Key + body) for assertions.
    async fn mock_stripe(handler: impl Fn(&str, &str) -> (u16, String) + Send + Sync + 'static) -> (String, Arc<Mutex<Vec<Seen>>>) {
        let seen: Arc<Mutex<Vec<Seen>>> = Arc::new(Mutex::new(vec![]));
        let seen2 = seen.clone();
        let handler = Arc::new(handler);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                let seen = seen2.clone();
                let handler = handler.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 65536];
                    let Ok(n) = sock.read(&mut buf).await else { return };
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let mut lines = req.lines();
                    let head = lines.next().unwrap_or("");
                    let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let mut key = String::new();
                    for l in lines.by_ref() {
                        if l.is_empty() {
                            break;
                        }
                        if let Some(v) = l.strip_prefix("Idempotency-Key:").or_else(|| l.strip_prefix("idempotency-key:")) {
                            key = v.trim().to_string();
                        }
                    }
                    let body: String = lines.collect::<Vec<_>>().join("\n");
                    // Strip query strings; Stripe paths have none here.
                    let (code, payload) = handler(&path, &body);
                    seen.lock().unwrap().push(Seen { method_path: path, idempotency_key: key, body });
                    let resp = format!(
                        "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        payload.len(),
                        payload
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        (base, seen)
    }

    fn req(action: PspAction, data: &str) -> PspRequest {
        PspRequest {
            action,
            amount: Decimal::new(1999, 2),
            currency: "USD".into(),
            idempotency_key: "test-key-1".into(),
            return_url: Some("https://shop/return".into()),
            data: Some(data.into()),
        }
    }

    #[tokio::test]
    async fn authorize_succeeded_completes_with_minor_units() {
        let (base, seen) = mock_stripe(|path, _| {
            assert!(path == "/v1/payment_intents", "{path}");
            (200, r#"{"id":"pi_123","status":"succeeded"}"#.into())
        })
        .await;
        let p = StripePsp::with_base("sk_test_x", base);
        let out = p.execute_async(&req(PspAction::Authorize, r#"{"payment_method":"pm_1"}"#)).await;
        assert!(matches!(&out, PspOutcome::Completed { psp_reference } if psp_reference == "pi_123"), "{out:?}");
        let s = seen.lock().unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].idempotency_key, "test-key-1");
        assert!(s[0].body.contains("amount=1999"), "minor units: {}", s[0].body);
        assert!(s[0].body.contains("capture_method=manual"), "{}", s[0].body);
    }

    #[tokio::test]
    async fn authorize_requires_action_maps_3ds_redirect() {
        let (base, _) = mock_stripe(|_, _| {
            (200, r#"{"id":"pi_3ds","status":"requires_action","next_action":{"type":"redirect_to_url","redirect_to_url":{"url":"https://hooks.stripe.com/3ds/abc"}}}"#.into())
        })
        .await;
        let p = StripePsp::with_base("sk_test_x", base);
        let out = p.execute_async(&req(PspAction::Authorize, r#"{"payment_method":"pm_1"}"#)).await;
        match out {
            PspOutcome::ActionRequired { psp_reference, redirect_url, .. } => {
                assert_eq!(psp_reference, "pi_3ds");
                assert_eq!(redirect_url, "https://hooks.stripe.com/3ds/abc");
            }
            other => panic!("want ActionRequired, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn stripe_error_becomes_failed_with_message() {
        let (base, _) = mock_stripe(|_, _| {
            (402, r#"{"error":{"message":"Your card was declined.","type":"card_error","code":"card_declined"}}"#.into())
        })
        .await;
        let p = StripePsp::with_base("sk_test_x", base);
        let out = p.execute_async(&req(PspAction::Authorize, r#"{"payment_method":"pm_1"}"#)).await;
        assert!(matches!(&out, PspOutcome::Failed { error } if error.contains("declined") && error.contains("card_declined")), "{out:?}");
    }

    #[tokio::test]
    async fn capture_and_refund_and_cancel_paths() {
        let (base, seen) = mock_stripe(|path, _| {
            if path == "/v1/payment_intents/pi_1/capture" {
                (200, r#"{"id":"pi_1","status":"succeeded"}"#.into())
            } else if path == "/v1/refunds" {
                (200, r#"{"id":"re_1","status":"succeeded"}"#.into())
            } else if path == "/v1/payment_intents/pi_1/cancel" {
                (200, r#"{"id":"pi_1","status":"canceled"}"#.into())
            } else {
                (404, r#"{"error":{"message":"nope"}}"#.into())
            }
        })
        .await;
        let p = StripePsp::with_base("sk_test_x", base);
        let c = p.execute_async(&req(PspAction::Charge, r#"{"payment_intent":"pi_1"}"#)).await;
        assert!(matches!(c, PspOutcome::Completed { .. }), "{c:?}");
        let r = p.execute_async(&req(PspAction::Refund, r#"{"payment_intent":"pi_1"}"#)).await;
        assert!(matches!(&r, PspOutcome::Completed { psp_reference } if psp_reference == "re_1"), "{r:?}");
        let x = p.execute_async(&req(PspAction::Cancel, r#"{"payment_intent":"pi_1"}"#)).await;
        // canceled intents carry no funds: terminal Failed (nothing moves).
        assert!(matches!(x, PspOutcome::Failed { .. }), "{x:?}");
        assert_eq!(seen.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn missing_payment_method_fails_cleanly_without_http() {
        let (base, seen) = mock_stripe(|_, _| (200, "{}".into())).await;
        let p = StripePsp::with_base("sk_test_x", base);
        let out = p.execute_async(&req(PspAction::Authorize, "{}")).await;
        assert!(matches!(&out, PspOutcome::Failed { error } if error.contains("payment_method")), "{out:?}");
        assert!(seen.lock().unwrap().is_empty(), "no HTTP without a payment method");
    }

    /// block_in_place needs the multi-thread runtime (the server runs one;
    /// single-thread `#[tokio::test]` would panic by design).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sync_bridge_used_by_engine() {
        use saleor_rustify_core::psp::Psp;
        let (base, _) = mock_stripe(|_, _| {
            (200, r#"{"id":"pi_sync","status":"succeeded"}"#.into())
        })
        .await;
        let p = StripePsp::with_base("sk_test_x", base);
        // Same dynamic call the engine makes (`&dyn Psp` → block_in_place).
        let out = (&p as &dyn Psp).execute(&req(PspAction::Charge, r#"{"payment_method":"pm_1"}"#));
        assert!(matches!(&out, PspOutcome::Completed { psp_reference } if psp_reference == "pi_sync"), "{out:?}");
    }

    #[test]
    fn minor_units_math() {
        assert_eq!(minor_units(Decimal::new(1999, 2), "USD").unwrap(), 1999);
        assert_eq!(minor_units(Decimal::new(1000, 0), "JPY").unwrap(), 1000);
        // Zero-decimal: 19.99 JPY rounds half-up to 20 yen.
        assert_eq!(minor_units(Decimal::new(1999, 2), "jpy").unwrap(), 20);
    }
}
