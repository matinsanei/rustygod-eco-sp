//! Authentication compatible with Django:
//! - password verification for Django's hashers (`pbkdf2_sha256` — what all
//!   Saleor rows use — plus `pbkdf2_sha1` and `bcrypt`), so existing users log
//!   in with **no password reset**;
//! - RS256 JWTs with Saleor's claim shape (`owner/iss/iat/exp/token/email/
//!   type/user_id/is_staff`), signed by the **same `RSA_PRIVATE_KEY`** Django
//!   reads — tokens verify on both sides.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordCheck {
    Ok,
    Wrong,
    UnsupportedHasher(String),
}

/// Verify a raw password against a Django-encoded hash.
/// Format: `<algo>$<iterations>$<salt>$<base64-hash>`.
pub fn verify_password(raw: &str, encoded: &str) -> PasswordCheck {
    let parts: Vec<&str> = encoded.split('$').collect();
    match parts.as_slice() {
        ["pbkdf2_sha256", iters, salt, hash] => {
            verify_pbkdf2_sha256(raw, iters, salt, hash)
        }
        ["pbkdf2_sha1", iters, salt, hash] => {
            verify_pbkdf2_sha1(raw, iters, salt, hash)
        }
        ["bcrypt", rest] => verify_bcrypt(raw, rest),
        ["bcrypt_sha256", rest] => verify_bcrypt(raw, rest),
        [algo, ..] => PasswordCheck::UnsupportedHasher(algo.to_string()),
        _ => PasswordCheck::UnsupportedHasher("malformed".to_string()),
    }
}

fn verify_pbkdf2_sha256(raw: &str, iters: &str, salt: &str, hash_b64: &str) -> PasswordCheck {
    use base64::Engine;
    let Ok(iters) = iters.parse::<u32>() else {
        return PasswordCheck::Wrong;
    };
    let Ok(expected) = base64::engine::general_purpose::STANDARD.decode(hash_b64) else {
        return PasswordCheck::Wrong;
    };
    let mut out = vec![0u8; expected.len()];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(raw.as_bytes(), salt.as_bytes(), iters, &mut out);
    if constant_time_eq(&out, &expected) {
        PasswordCheck::Ok
    } else {
        PasswordCheck::Wrong
    }
}

fn verify_pbkdf2_sha1(raw: &str, iters: &str, salt: &str, hash_b64: &str) -> PasswordCheck {
    use base64::Engine;
    let Ok(iters) = iters.parse::<u32>() else {
        return PasswordCheck::Wrong;
    };
    let Ok(expected) = base64::engine::general_purpose::STANDARD.decode(hash_b64) else {
        return PasswordCheck::Wrong;
    };
    let mut out = vec![0u8; expected.len()];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(raw.as_bytes(), salt.as_bytes(), iters, &mut out);
    if constant_time_eq(&out, &expected) {
        PasswordCheck::Ok
    } else {
        PasswordCheck::Wrong
    }
}

fn verify_bcrypt(raw: &str, rest: &str) -> PasswordCheck {
    // Django stores `bcrypt$<modular-crypt-string>`; the `$` split above
    // leaves the rest re-joined with `$`.
    let hash = format!("${rest}");
    match bcrypt::verify(raw, &hash) {
        Ok(true) => PasswordCheck::Ok,
        Ok(false) => PasswordCheck::Wrong,
        Err(_) => PasswordCheck::Wrong,
    }
}

/// Hash a raw token/password in Django's `pbkdf2_sha256` format
/// (`pbkdf2_sha256$<iters>$<salt>$<base64>`), readable by both Django's
/// `make_password`/`check_password` and our `verify_password`.
/// Mirrors `AppToken.set_auth_token` in `saleor/app/models.py`.
pub fn hash_password(raw: &str) -> String {
    use base64::Engine;
    use rand::Rng;
    const ITERS: u32 = 600_000;
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let salt: String = (0..12)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect();
    let mut out = vec![0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(raw.as_bytes(), salt.as_bytes(), ITERS, &mut out);
    format!(
        "pbkdf2_sha256${ITERS}${salt}${}",
        base64::engine::general_purpose::STANDARD.encode(&out)
    )
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

/// Saleor global id: base64("User:7").
pub fn user_global_id(user_id: i32) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(format!("User:{user_id}"))
}

pub fn parse_user_global_id(gid: &str) -> Option<i32> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(gid).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    s.strip_prefix("User:")?.parse::<i32>().ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub iat: i64,
    pub exp: i64,
    pub iss: String,
    pub owner: String,
    pub token: String,
    pub email: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub user_id: String,
    pub is_staff: bool,
}

pub const TOKEN_TYPE_ACCESS: &str = "access";
pub const TOKEN_TYPE_REFRESH: &str = "refresh";

/// Access lives 5 minutes, refresh 30 days — Django's `JWT_TTL_*` defaults.
pub const ACCESS_TTL_SECS: i64 = 5 * 60;
pub const REFRESH_TTL_SECS: i64 = 30 * 24 * 3600;

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("no RSA private key (set RSA_PRIVATE_KEY like Django)")]
    NoKey,
    #[error("bad key: {0}")]
    BadKey(String),
    #[error("invalid token")]
    Invalid,
    #[error("token expired")]
    Expired,
}

fn encoding_key_from(pem: &str) -> Result<jsonwebtoken::EncodingKey, JwtError> {
    jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| JwtError::BadKey(e.to_string()))
}

fn encoding_key() -> Result<jsonwebtoken::EncodingKey, JwtError> {
    let pem = std::env::var("RSA_PRIVATE_KEY").map_err(|_| JwtError::NoKey)?;
    encoding_key_from(&pem)
}

pub struct TokenPair {
    pub access: String,
    pub refresh: String,
}

#[allow(clippy::too_many_arguments)]
pub fn mint_tokens(
    issuer: &str,
    email: &str,
    user_id: i32,
    is_staff: bool,
    jwt_token_key: &str,
) -> Result<TokenPair, JwtError> {
    let pem = std::env::var("RSA_PRIVATE_KEY").map_err(|_| JwtError::NoKey)?;
    mint_tokens_with_key(&pem, issuer, email, user_id, is_staff, jwt_token_key)
}

#[allow(clippy::too_many_arguments)]
pub fn mint_tokens_with_key(
    pem: &str,
    issuer: &str,
    email: &str,
    user_id: i32,
    is_staff: bool,
    jwt_token_key: &str,
) -> Result<TokenPair, JwtError> {
    let key = encoding_key_from(pem)?;
    let now = chrono::Utc::now().timestamp();
    let mk = |ttype: &str, ttl: i64| {
        let claims = Claims {
            iat: now,
            exp: now + ttl,
            iss: issuer.to_string(),
            owner: "saleor".to_string(),
            token: jwt_token_key.to_string(),
            email: email.to_string(),
            token_type: ttype.to_string(),
            user_id: user_global_id(user_id),
            is_staff,
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
            &claims,
            &key,
        )
        .map_err(|e| JwtError::BadKey(format!("encode: {e:?}")))
    };
    Ok(TokenPair {
        access: mk(TOKEN_TYPE_ACCESS, ACCESS_TTL_SECS)?,
        refresh: mk(TOKEN_TYPE_REFRESH, REFRESH_TTL_SECS)?,
    })
}

pub fn decode(token: &str) -> Result<Claims, JwtError> {
    let pem = std::env::var("RSA_PRIVATE_KEY").map_err(|_| JwtError::NoKey)?;
    decode_with_key(token, &pem)
}

fn decoding_key_from(pem: &str) -> Result<jsonwebtoken::DecodingKey, JwtError> {
    if let Ok(pub_pem) = std::env::var("RSA_PUBLIC_KEY") {
        return jsonwebtoken::DecodingKey::from_rsa_pem(pub_pem.as_bytes())
            .map_err(|e| JwtError::BadKey(e.to_string()));
    }
    use base64::Engine;
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::traits::PublicKeyParts;
    let priv_key = rsa::RsaPrivateKey::from_pkcs8_pem(pem)
        .map_err(|e| JwtError::BadKey(e.to_string()))?;
    let n = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(priv_key.n().to_bytes_be());
    let e = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(priv_key.e().to_bytes_be());
    jsonwebtoken::DecodingKey::from_rsa_components(&n, &e)
        .map_err(|e| JwtError::BadKey(e.to_string()))
}

pub fn decode_with_key(token: &str, pem: &str) -> Result<Claims, JwtError> {
    let key = decoding_key_from(pem)?;
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.validate_exp = true;
    validation.set_audience(&[] as &[&str]);
    // Issuer checked by caller domain config; exp enforced here.
    validation.validate_aud = false;
    jsonwebtoken::decode::<Claims>(token, &key, &validation)
        .map(|d| d.claims)
        .map_err(|e| JwtError::BadKey(format!("decode: {e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_id_roundtrip() {
        assert_eq!(parse_user_global_id(&user_global_id(7)), Some(7));
        assert_eq!(parse_user_global_id("!!!"), None);
    }

    #[test]
    fn wrong_password_rejected_fast() {
        // Format-level checks without a real hash.
        assert!(matches!(
            verify_password("x", "nonsense"),
            PasswordCheck::UnsupportedHasher(_)
        ));
    }

    #[test]
    fn hash_roundtrips_through_verify() {
        let h = hash_password("s3cret-token-value");
        assert!(h.starts_with("pbkdf2_sha256$600000$"));
        assert!(matches!(verify_password("s3cret-token-value", &h), PasswordCheck::Ok));
        assert!(matches!(verify_password("wrong", &h), PasswordCheck::Wrong));
        // Salts differ per call.
        assert_ne!(h, hash_password("s3cret-token-value"));
    }
}
