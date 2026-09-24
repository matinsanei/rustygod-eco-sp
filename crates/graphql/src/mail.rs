//! Minimal SMTP client (Django `send_mail` parity for transactional mail).
//!
//! No third-party mailer is vendored: plain SMTP (EHLO → MAIL → RCPT →
//! DATA → QUIT) over Tokio, enough for Mailpit/local relays and plain
//! submission servers. TLS/auth are explicit non-goals (documented —
//! production deployments front a relay on localhost, exactly what
//! `EMAIL_HOST=127.0.0.1` means in Django settings).
//!
//! Env: `RUSTIFY_SMTP_HOST` (default `127.0.0.1`), `RUSTIFY_SMTP_PORT`
//! (default `1025`), `RUSTIFY_MAIL_FROM` (default
//! `noreply@saleor-rustify.local`). Failures are returned, never panicked;
//! callers log-and-continue so a down mailer can't break registration.

use async_graphql::Error;

type Result<T> = std::result::Result<T, Error>;

fn mail_err(msg: impl Into<String>) -> Error {
    Error::new(format!("mail error: {}", msg.into()))
}
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub from: String,
}

impl SmtpConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("RUSTIFY_SMTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("RUSTIFY_SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(1025),
            from: std::env::var("RUSTIFY_MAIL_FROM")
                .unwrap_or_else(|_| "noreply@saleor-rustify.local".to_string()),
        }
    }
}

/// Send one plain-text email. Subject/body are header-sanitized (no CRLF
/// injection — Django's `sanitize_address` equivalent for our fields).
pub async fn send_mail(to: &str, subject: &str, body: &str) -> Result<()> {
    send_mail_with(SmtpConfig::from_env(), to, subject, body).await
}

pub async fn send_mail_with(cfg: SmtpConfig, to: &str, subject: &str, body: &str) -> Result<()> {
    let to = to.trim();
    if to.is_empty() || !to.contains('@') {
        return Err(mail_err("invalid recipient email"));
    }
    let clean = |s: &str| s.replace(['\r', '\n'], " ");
    let subject = clean(subject);
    let date = chrono::Utc::now().to_rfc2822();
    let msg = format!(
        "From: {}\r\nTo: {to}\r\nSubject: {subject}\r\nDate: {date}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}\r\n",
        cfg.from
    );
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let stream = tokio::net::TcpStream::connect((cfg.host.as_str(), cfg.port))
        .await
        .map_err(|e| mail_err(format!("smtp connect failed: {e}")))?;
    let (rh, mut wh) = stream.into_split();
    let mut rd = BufReader::new(rh);
    let mut line = String::new();
    let mut greeting = String::new();
    rd.read_line(&mut greeting).await.map_err(|e| mail_err(format!("smtp read failed: {e}")))?;
    if !greeting.starts_with("220") {
        return Err(mail_err(format!("smtp bad greeting: {}", greeting.trim())));
    }
    let from = cfg.from.clone();
    smtp_cmd(&mut wh, &mut rd, &mut line, "EHLO saleor-rustify\r\n", "250").await?;
    smtp_cmd(&mut wh, &mut rd, &mut line, &format!("MAIL FROM:<{from}>\r\n"), "250").await?;
    smtp_cmd(&mut wh, &mut rd, &mut line, &format!("RCPT TO:<{to}>\r\n"), "250").await?;
    smtp_cmd(&mut wh, &mut rd, &mut line, "DATA\r\n", "354").await?;
    // Dot-stuff the body, then terminate.
    let stuffed = msg.replace("\r\n.", "\r\n..");
    smtp_cmd(&mut wh, &mut rd, &mut line, &format!("{stuffed}\r\n.\r\n"), "250").await?;
    smtp_cmd(&mut wh, &mut rd, &mut line, "QUIT\r\n", "221").await?;
    Ok(())
}

/// One SMTP command round-trip (free fn so borrows end each await).
async fn smtp_cmd(
    wh: &mut tokio::net::tcp::OwnedWriteHalf,
    rd: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
    line: &mut String,
    s: &str,
    want: &str,
) -> std::result::Result<(), Error> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    wh.write_all(s.as_bytes()).await.map_err(|e| mail_err(format!("smtp write failed: {e}")))?;
    line.clear();
    rd.read_line(line).await.map_err(|e| mail_err(format!("smtp read failed: {e}")))?;
    if !line.starts_with(want) {
        return Err(mail_err(format!("smtp unexpected reply: {}", line.trim())));
    }
    Ok(())
}

/// Confirmation email (Django `send_account_confirmation_email`): link
/// carries the confirm token; redirect base comes from the storefront.
pub async fn send_confirmation(to: &str, redirect_url: &str, token: &str) -> Result<()> {
    let link = format!("{}/confirm?token={token}", redirect_url.trim_end_matches('/'));
    send_mail(
        to,
        "Confirm your account",
        &format!("Welcome! Confirm your account by visiting:\n\n{link}\n\nThis link expires in 7 days."),
    )
    .await
}

/// Password-reset email (Django `send_password_reset_email`).
pub async fn send_password_reset(to: &str, redirect_url: &str, token: &str, email: &str) -> Result<()> {
    let link = format!("{}/reset-password?token={token}&email={email}", redirect_url.trim_end_matches('/'));
    send_mail(
        to,
        "Reset your password",
        &format!("Reset your password by visiting:\n\n{link}\n\nThis link expires in 24 hours."),
    )
    .await
}
