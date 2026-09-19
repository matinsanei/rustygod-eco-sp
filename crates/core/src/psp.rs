//! PSP abstraction: how money actually moves.
//!
//! Mirrors `saleor/payment/interface.py::GatewayResponse` and the
//! transaction-item event contract (`saleor/payment/__init__.py::
//! TransactionEventType`):
//! - a PSP never touches the DB; it only answers "what happened?" for one
//!   action. `db::payments` turns the answer into events (request/success,
//!   pending, or action_required) — exactly how Saleor's payment app
//!   webhooks feed `TransactionEvent` rows;
//! - outcomes: Completed (sync, funds moved), Pending (async, result later
//!   via callback), ActionRequired (3DS/customer challenge — Saleor's
//!   `authorization_action_required` / `charge_action_required` events,
//!   redirect URL in `external_url`), Failed (terminal, no event).
//!
//! Implementations here are local sims (deterministic tests, storefront
//! 3DS rehearsal). A real PSP (Stripe/Adyen HTTP) is a new crate
//! implementing [`Psp`] — the event contract below doesn't change.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

/// The four money actions, matching the manual-gateway verbs in
/// `db::payments` and Saleor's `TransactionAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PspAction {
    Authorize,
    Charge,
    Refund,
    Cancel,
}

impl PspAction {
    /// Event types this action writes on the happy path.
    pub fn request_event(&self) -> &'static str {
        match self {
            PspAction::Authorize => "authorization_request",
            PspAction::Charge => "charge_request",
            PspAction::Refund => "refund_request",
            PspAction::Cancel => "cancel_request",
        }
    }
    pub fn success_event(&self) -> &'static str {
        match self {
            PspAction::Authorize => "authorization_success",
            PspAction::Charge => "charge_success",
            PspAction::Refund => "refund_success",
            PspAction::Cancel => "cancel_success",
        }
    }
    pub fn failure_event(&self) -> &'static str {
        match self {
            PspAction::Authorize => "authorization_failure",
            PspAction::Charge => "charge_failure",
            PspAction::Refund => "refund_failure",
            PspAction::Cancel => "cancel_failure",
        }
    }
    /// 3DS-style challenge event. Saleor defines action_required only for
    /// authorize/charge (`get_correct_event_types_based_on_request_type`
    /// has no action_required answer for refund/cancel) — same here.
    pub fn action_required_event(&self) -> Option<&'static str> {
        match self {
            PspAction::Authorize => Some("authorization_action_required"),
            PspAction::Charge => Some("charge_action_required"),
            PspAction::Refund | PspAction::Cancel => None,
        }
    }
    /// Valid terminal answers to a callback for a pending `*_request`,
    /// port of Django's `get_correct_event_types_based_on_request_type`.
    pub fn callback_answers(&self) -> &'static [&'static str] {
        match self {
            PspAction::Authorize => &["authorization_success", "authorization_failure"],
            PspAction::Charge => &["charge_success", "charge_failure"],
            PspAction::Refund => &["refund_success", "refund_failure"],
            PspAction::Cancel => &["cancel_success", "cancel_failure"],
        }
    }
}

/// One money action the PSP must answer.
#[derive(Debug, Clone)]
pub struct PspRequest {
    pub action: PspAction,
    pub amount: Decimal,
    pub currency: String,
    pub idempotency_key: String,
    /// Where the customer returns after a challenge (3DS `return_url`).
    pub return_url: Option<String>,
}

/// What the PSP says. Mirrors `GatewayResponse` fields that matter:
/// `is_success` → Completed/Failed, `action_required` → ActionRequired,
/// async (no immediate answer) → Pending.
#[derive(Debug, Clone)]
pub enum PspOutcome {
    Completed { psp_reference: String },
    Pending { psp_reference: String },
    ActionRequired { psp_reference: String, redirect_url: String, message: String },
    Failed { error: String },
}

pub trait Psp: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, req: &PspRequest) -> PspOutcome;
}

/// Saleor's manual/dummy gateway: funds move immediately, always.
pub struct ManualPsp;

impl Psp for ManualPsp {
    fn name(&self) -> &'static str {
        "manual"
    }
    fn execute(&self, req: &PspRequest) -> PspOutcome {
        PspOutcome::Completed {
            psp_reference: format!("manual-{}-{}", req.action_string(), req.idempotency_key),
        }
    }
}

impl PspRequest {
    fn action_string(&self) -> &'static str {
        match self.action {
            PspAction::Authorize => "auth",
            PspAction::Charge => "charge",
            PspAction::Refund => "refund",
            PspAction::Cancel => "cancel",
        }
    }
}

/// Always answers "go ask the customer": 3DS rehearsal for the storefront
/// without any async machinery. The redirect URL is fake but well-formed
/// (carries psp_reference + return URL) so the full challenge loop can be
/// exercised end to end.
pub struct ChallengePsp {
    pub challenge_host: String,
}

impl ChallengePsp {
    pub fn new(host: impl Into<String>) -> Self {
        Self { challenge_host: host.into() }
    }
}

impl Psp for ChallengePsp {
    fn name(&self) -> &'static str {
        "challenge"
    }
    fn execute(&self, req: &PspRequest) -> PspOutcome {
        if req.action.action_required_event().is_none() {
            // Django parity: no action_required answer for refund/cancel.
            return PspOutcome::Failed {
                error: format!("{:?} does not support customer challenges", req.action),
            };
        }
        let psp_reference = format!("chall-{}-{}", req.action_string(), req.idempotency_key);
        let back = req.return_url.clone().unwrap_or_else(|| "rustygod://3ds-return".into());
        PspOutcome::ActionRequired {
            redirect_url: format!(
                "https://{}/3ds/challenge?psp={psp_reference}&return_url={back}",
                self.challenge_host
            ),
            message: format!("3DS challenge required for {} {}", req.amount, req.currency),
            psp_reference,
        }
    }
}

/// Scripted answers for deterministic tests: pop the next outcome, or the
/// default when the script is exhausted. Interior mutability so one PSP can
/// be shared across calls.
pub struct ScriptedPsp {
    name: &'static str,
    script: Mutex<VecDeque<PspOutcome>>,
    default: PspOutcome,
}

impl ScriptedPsp {
    pub fn pending(name: &'static str) -> Self {
        Self {
            name,
            script: Mutex::new(VecDeque::new()),
            default: PspOutcome::Pending { psp_reference: "script-pending".into() },
        }
    }
    pub fn push(&self, outcome: PspOutcome) {
        self.script.lock().unwrap().push_back(outcome);
    }
}

impl Psp for ScriptedPsp {
    fn name(&self) -> &'static str {
        self.name
    }
    fn execute(&self, _req: &PspRequest) -> PspOutcome {
        self.script.lock().unwrap().pop_front().unwrap_or_else(|| self.default.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(action: PspAction) -> PspRequest {
        PspRequest {
            action,
            amount: Decimal::new(1000, 2),
            currency: "USD".into(),
            idempotency_key: "k".into(),
            return_url: None,
        }
    }

    #[test]
    fn manual_always_completes() {
        let p = ManualPsp;
        for a in [PspAction::Authorize, PspAction::Charge, PspAction::Refund, PspAction::Cancel] {
            assert!(matches!(p.execute(&req(a)), PspOutcome::Completed { .. }), "{a:?}");
        }
    }

    #[test]
    fn challenge_only_for_auth_and_charge() {
        let p = ChallengePsp::new("3ds.test");
        assert!(matches!(
            p.execute(&req(PspAction::Authorize)),
            PspOutcome::ActionRequired { .. }
        ));
        let out = p.execute(&req(PspAction::Refund));
        assert!(matches!(out, PspOutcome::Failed { .. }), "refund has no 3DS answer: {out:?}");
        if let PspOutcome::ActionRequired { redirect_url, .. } =
            p.execute(&req(PspAction::Charge))
        {
            assert!(redirect_url.starts_with("https://3ds.test/3ds/challenge?"));
        } else {
            panic!("charge must challenge");
        }
    }

    #[test]
    fn scripted_pops_then_defaults() {
        let p = ScriptedPsp::pending("sim");
        p.push(PspOutcome::Failed { error: "no funds".into() });
        assert!(matches!(p.execute(&req(PspAction::Charge)), PspOutcome::Failed { .. }));
        assert!(matches!(p.execute(&req(PspAction::Charge)), PspOutcome::Pending { .. }));
    }

    #[test]
    fn callback_answer_map_covers_all_actions() {
        // Port of get_correct_event_types_based_on_request_type: every
        // action has exactly its success/failure pair as valid answers.
        for a in [PspAction::Authorize, PspAction::Charge, PspAction::Refund, PspAction::Cancel] {
            let ans = a.callback_answers();
            assert_eq!(ans.len(), 2);
            assert!(ans[0].ends_with("_success") && ans[1].ends_with("_failure"));
            assert_eq!(a.success_event(), ans[0]);
            assert_eq!(a.failure_event(), ans[1]);
        }
    }
}
