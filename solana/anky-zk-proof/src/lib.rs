use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const PROTOCOL: &str = "ANKY_ZK_PROOF_V0";
pub const VERSION: u16 = 1;
pub const TERMINAL_LINE: &str = "8000";
pub const TERMINAL_RECORD: &str = "\n8000";
pub const MAX_DELTA_MS: i64 = 7_999;
pub const DELTA_WIDTH: usize = 4;
pub const TERMINAL_SILENCE_MS: i64 = 8_000;
pub const FULL_ANKY_DURATION_MS: i64 = 8 * 60 * 1_000;
pub const MS_PER_UTC_DAY: i64 = 86_400_000;
pub const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
pub const SPACE_TOKEN: &str = "SPACE";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAnky {
    pub closed: bool,
    pub started_at_ms: i64,
    pub last_accepted_at_ms: i64,
    pub event_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProofReceipt {
    pub version: u16,
    pub protocol: String,
    pub writer: String,
    pub session_hash: String,
    pub utc_day: i64,
    pub started_at_ms: i64,
    pub accepted_duration_ms: i64,
    pub rite_duration_ms: i64,
    pub event_count: usize,
    pub valid: bool,
    pub duration_ok: bool,
    pub proof_hash: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProofError {
    #[error("invalid .anky: {0}")]
    InvalidAnky(String),
    #[error("writer must be a non-empty wallet/public identity")]
    EmptyWriter,
    #[error("invalid expected hash `{0}`: expected 64 lowercase or uppercase hex characters")]
    InvalidExpectedHash(String),
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("session is shorter than a full Anky: rite_duration_ms {actual} < {minimum}")]
    DurationTooShort { actual: i64, minimum: i64 },
}

pub fn build_receipt(
    raw: &str,
    writer: &str,
    expected_session_hash: Option<&str>,
) -> Result<ProofReceipt, ProofError> {
    let writer = writer.trim();
    if writer.is_empty() {
        return Err(ProofError::EmptyWriter);
    }

    let parsed = parse_anky(raw)?;
    let session_hash = compute_session_hash(raw);

    if let Some(expected) = expected_session_hash {
        let expected = normalize_expected_hash(expected)?;
        if expected != session_hash {
            return Err(ProofError::HashMismatch {
                expected,
                actual: session_hash,
            });
        }
    }

    let accepted_duration_ms = parsed.last_accepted_at_ms - parsed.started_at_ms;
    let rite_duration_ms = accepted_duration_ms + TERMINAL_SILENCE_MS;
    if rite_duration_ms < FULL_ANKY_DURATION_MS {
        return Err(ProofError::DurationTooShort {
            actual: rite_duration_ms,
            minimum: FULL_ANKY_DURATION_MS,
        });
    }

    let utc_day = parsed.started_at_ms.div_euclid(MS_PER_UTC_DAY);
    let proof_hash = compute_receipt_hash(
        writer,
        &session_hash,
        utc_day,
        parsed.started_at_ms,
        accepted_duration_ms,
        rite_duration_ms,
        parsed.event_count,
        true,
    );

    Ok(ProofReceipt {
        version: VERSION,
        protocol: PROTOCOL.to_string(),
        writer: writer.to_string(),
        session_hash,
        utc_day,
        started_at_ms: parsed.started_at_ms,
        accepted_duration_ms,
        rite_duration_ms,
        event_count: parsed.event_count,
        valid: true,
        duration_ok: true,
        proof_hash,
    })
}

pub fn parse_anky(raw: &str) -> Result<ParsedAnky, ProofError> {
    if raw.is_empty() {
        return Err(ProofError::InvalidAnky("File is empty.".to_string()));
    }

    let mut errors = Vec::new();

    if raw.starts_with('\u{feff}') {
        errors.push("File must not start with a BOM.".to_string());
    }

    if raw.contains('\r') {
        errors.push("Line endings must be LF only.".to_string());
    }

    let closed = raw.ends_with(TERMINAL_RECORD);
    if !closed {
        errors.push("Missing terminal 8000 line.".to_string());
    }

    let event_raw = if closed {
        &raw[..raw.len() - TERMINAL_RECORD.len()]
    } else {
        raw
    };
    let event_text = event_raw.strip_suffix('\n').unwrap_or(event_raw);
    let event_lines: Vec<&str> = event_text.split('\n').collect();

    let mut started_at = None;
    let mut accepted_at = None;
    let mut event_count = 0;

    for (index, line) in event_lines.iter().enumerate() {
        if line.is_empty() {
            errors.push(format!("Line {} is empty.", index + 1));
            continue;
        }

        if index == 0 {
            match parse_first_line(line) {
                Ok((epoch_ms, _char)) => {
                    started_at = Some(epoch_ms);
                    accepted_at = Some(epoch_ms);
                    event_count += 1;
                }
                Err(error) => errors.push(format!("Line 1: {error}")),
            }
            continue;
        }

        match parse_delta_line(line) {
            Ok((delta_ms, _char)) => {
                let next_accepted_at = accepted_at.unwrap_or(0) + delta_ms;
                accepted_at = Some(next_accepted_at);
                event_count += 1;
            }
            Err(error) => errors.push(format!("Line {}: {error}", index + 1)),
        }
    }

    if errors.is_empty() && event_count == 0 {
        errors.push("Session must contain at least one accepted character.".to_string());
    }

    if !errors.is_empty() {
        return Err(ProofError::InvalidAnky(errors.join("; ")));
    }

    let started_at_ms = started_at.ok_or_else(|| {
        ProofError::InvalidAnky("Session must contain a valid first line.".to_string())
    })?;
    let last_accepted_at_ms = accepted_at.unwrap_or(started_at_ms);

    Ok(ParsedAnky {
        closed,
        started_at_ms,
        last_accepted_at_ms,
        event_count,
    })
}

pub fn compute_session_hash(raw: &str) -> String {
    hex_sha256(raw.as_bytes())
}

pub fn compute_receipt_hash(
    writer: &str,
    session_hash: &str,
    utc_day: i64,
    started_at_ms: i64,
    accepted_duration_ms: i64,
    rite_duration_ms: i64,
    event_count: usize,
    duration_ok: bool,
) -> String {
    let payload = format!(
        "{PROTOCOL}|{VERSION}|{writer}|{session_hash}|{utc_day}|{started_at_ms}|{accepted_duration_ms}|{rite_duration_ms}|{event_count}|{duration_ok}"
    );

    hex_sha256(payload.as_bytes())
}

fn parse_first_line(line: &str) -> Result<(i64, String), String> {
    let separator_index = line
        .find(' ')
        .ok_or_else(|| "First line must be `{epoch_ms} {character}`.".to_string())?;
    if separator_index == 0 {
        return Err("First line must be `{epoch_ms} {character}`.".to_string());
    }

    let epoch = &line[..separator_index];
    let token = &line[separator_index + 1..];

    if !epoch.as_bytes().iter().all(u8::is_ascii_digit) {
        return Err("Epoch must contain only digits.".to_string());
    }

    let parsed_char = parse_character_token(token)?;
    let epoch_ms = epoch
        .parse::<i64>()
        .map_err(|_| "Epoch is not a safe integer.".to_string())?;
    if !(0..=MAX_SAFE_INTEGER).contains(&epoch_ms) {
        return Err("Epoch is not a safe integer.".to_string());
    }

    Ok((epoch_ms, parsed_char))
}

fn parse_delta_line(line: &str) -> Result<(i64, String), String> {
    if line.len() < DELTA_WIDTH + 2 || line.as_bytes().get(DELTA_WIDTH) != Some(&b' ') {
        return Err("Delta line must be `{delta_ms} {character}`.".to_string());
    }

    let delta = line
        .get(..DELTA_WIDTH)
        .ok_or_else(|| "Delta line must be `{delta_ms} {character}`.".to_string())?;
    let token = line
        .get(DELTA_WIDTH + 1..)
        .ok_or_else(|| "Delta line must be `{delta_ms} {character}`.".to_string())?;

    if !delta.as_bytes().iter().all(u8::is_ascii_digit) {
        return Err("Delta must be exactly four digits.".to_string());
    }

    let delta_ms = delta
        .parse::<i64>()
        .map_err(|_| "Delta must be exactly four digits.".to_string())?;
    if delta_ms > MAX_DELTA_MS {
        return Err("Delta must be capped at 7999.".to_string());
    }

    let parsed_char = parse_character_token(token)?;

    Ok((delta_ms, parsed_char))
}

fn parse_character_token(token: &str) -> Result<String, String> {
    if token == SPACE_TOKEN {
        return Ok(" ".to_string());
    }

    if token == " " {
        return Err("Space must be encoded as SPACE.".to_string());
    }

    if !is_accepted_character(token) {
        return Err("Character is not an accepted single character or SPACE token.".to_string());
    }

    Ok(token.to_string())
}

fn is_accepted_character(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(character) = chars.next() else {
        return false;
    };
    if chars.next().is_some() {
        return false;
    }

    let code_point = character as u32;
    code_point > 31 && code_point != 127
}

fn normalize_expected_hash(input: &str) -> Result<String, ProofError> {
    let trimmed = input.trim();
    if trimmed.len() != 64 || !trimmed.as_bytes().iter().all(u8::is_ascii_hexdigit) {
        return Err(ProofError::InvalidExpectedHash(input.to_string()));
    }

    Ok(trimmed.to_ascii_lowercase())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRITER: &str = "8qznzSWh7vzM2G1JrDUhEYrPpZK2ehDUmydQiFpU8Q19";

    #[test]
    fn builds_receipt_for_full_closed_session() {
        let raw = full_anky_raw();

        let receipt = build_receipt(&raw, WRITER, None).expect("receipt");

        assert_eq!(receipt.version, 1);
        assert_eq!(receipt.protocol, PROTOCOL);
        assert_eq!(receipt.writer, WRITER);
        assert_eq!(receipt.session_hash, compute_session_hash(&raw));
        assert_eq!(
            receipt.utc_day,
            1_700_000_000_000_i64.div_euclid(MS_PER_UTC_DAY)
        );
        assert_eq!(receipt.started_at_ms, 1_700_000_000_000);
        assert_eq!(receipt.accepted_duration_ms, 60 * MAX_DELTA_MS);
        assert_eq!(
            receipt.rite_duration_ms,
            (60 * MAX_DELTA_MS) + TERMINAL_SILENCE_MS
        );
        assert_eq!(receipt.event_count, 61);
        assert!(receipt.valid);
        assert!(receipt.duration_ok);
        assert_eq!(receipt.proof_hash.len(), 64);
    }

    #[test]
    fn rejects_missing_terminal_line() {
        let error = parse_anky("1700000000000 a\n0001 b\n").expect_err("parse error");

        assert!(error.to_string().contains("Missing terminal 8000 line."));
    }

    #[test]
    fn rejects_literal_space_character_token() {
        let error = parse_anky("1700000000000 a\n0001  \n8000").expect_err("parse error");

        assert!(error
            .to_string()
            .contains("Space must be encoded as SPACE."));
    }

    #[test]
    fn accepts_space_token_and_single_unicode_scalar() {
        let parsed = parse_anky("1700000000000 SPACE\n0001 ね\n8000").expect("parse");

        assert!(parsed.closed);
        assert_eq!(parsed.event_count, 2);
        assert_eq!(parsed.last_accepted_at_ms, 1_700_000_000_001);
    }

    #[test]
    fn rejects_hash_mismatch() {
        let error = build_receipt(&full_anky_raw(), WRITER, Some(&"0".repeat(64)))
            .expect_err("hash mismatch");

        assert!(matches!(error, ProofError::HashMismatch { .. }));
    }

    #[test]
    fn rejects_short_session() {
        let raw = "1700000000000 a\n0001 b\n8000";
        let error = build_receipt(raw, WRITER, None).expect_err("too short");

        assert_eq!(
            error,
            ProofError::DurationTooShort {
                actual: 8_001,
                minimum: FULL_ANKY_DURATION_MS,
            }
        );
    }

    fn full_anky_raw() -> String {
        let mut raw = "1700000000000 a\n".to_string();
        for _ in 0..60 {
            raw.push_str("7999 a\n");
        }
        raw.push_str(TERMINAL_LINE);
        raw
    }
}
