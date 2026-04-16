use serde::{Deserialize, Serialize};

pub const CANONICAL_ANKY_SUBMIT_PATH: &str = "/api/anky/submit";
pub const CANONICAL_ANKY_SESSIONS_PATH_PREFIX: &str = "/api/anky/sessions";
pub const CANONICAL_CORE_LOOP: &str = "write -> title -> reflect -> image -> prove -> archive";
pub const CANONICAL_ANKY_MIN_DURATION_SECONDS: i64 = 480;
pub const CANONICAL_ANKY_MIN_WORD_COUNT: i32 = 300;
pub const CANONICAL_TITLE_WORD_COUNT: usize = 3;
pub const CANONICAL_FALLBACK_TITLE: &str = "untitled sacred reflection";
pub const LEGACY_SEALED_WRITE_MIN_WORD_COUNT: i32 = 50;

#[derive(Debug, Clone, Deserialize)]
pub struct AnkySessionBundleSubmitRequest {
    pub session_hash: String,
    pub session: String,
    pub duration_seconds: i64,
    pub word_count: i32,
    pub kingdom: String,
    pub started_at: String,
    #[serde(default)]
    pub wallet_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkySubmitAcceptedResponse {
    pub ok: bool,
    pub accepted: bool,
    pub is_anky: bool,
    pub canonical_path: &'static str,
    pub core_loop: &'static str,
    pub session_hash: String,
    pub anky_id: String,
    pub writing_session_id: String,
    pub status_path: String,
    pub proof_path: String,
    pub core_loop_status: CanonicalCoreLoopStatus,
    pub artifact_completeness: CompletedAnkyArtifactValidation,
    pub artifact_set_valid: bool,
    pub proof_metadata: ProofOfWritingMetadata,
    pub reflection_status: String,
    pub image_status: String,
    // Transitional compatibility field until the proof cutover replaces the
    // current `solana_status` column and downstream consumers.
    pub solana_status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnkySubmitEventName {
    Accepted,
    Title,
    ReflectionChunk,
    ReflectionComplete,
    ImageUrl,
    Proof,
    Done,
    Error,
}

impl AnkySubmitEventName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Title => "title",
            Self::ReflectionChunk => "reflection_chunk",
            Self::ReflectionComplete => "reflection_complete",
            Self::ImageUrl => "image_url",
            Self::Proof => "proof",
            Self::Done => "done",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalCoreLoopStatus {
    Accepted,
    Reflecting,
    Imaging,
    Proving,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalProofStatus {
    Pending,
    InProgress,
    Complete,
    Failed,
    Skipped,
}

impl CanonicalProofStatus {
    pub fn from_legacy_solana_status(status: &str) -> Self {
        match status.trim() {
            "in_progress" => Self::InProgress,
            "complete" => Self::Complete,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            _ => Self::Pending,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Failed | Self::Skipped)
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProofOfWritingAnchorKind {
    SolanaTransaction,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProofOfWritingReceipt {
    pub anchor_kind: ProofOfWritingAnchorKind,
    pub anchor_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProofOfWritingMetadata {
    pub session_hash: String,
    pub proof_status: CanonicalProofStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<ProofOfWritingReceipt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorReadbackPaths {
    pub status_path: String,
    pub proof_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorSessionIdentity {
    pub session_hash: String,
    pub anky_id: String,
    pub writing_session_id: String,
    pub duration_seconds: i64,
    pub word_count: i32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorLifecycleTimestamps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflection_started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflection_completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorStatusSnapshot {
    pub core_loop_status: CanonicalCoreLoopStatus,
    pub reflection_status: String,
    pub image_status: String,
    pub processing_job_state: String,
    pub proof_status: CanonicalProofStatus,
    pub artifact_completeness: CompletedAnkyArtifactValidation,
    pub artifact_set_valid: bool,
    pub timestamps: CanonicalProcessorLifecycleTimestamps,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorImageArtifact {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorArtifacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reflection: Option<String>,
    pub image: CanonicalProcessorImageArtifact,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProofReadback {
    pub session_hash: String,
    pub proof_status: CanonicalProofStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<ProofOfWritingReceipt>,
    pub artifact_status: ArtifactValidationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LegacyProcessorRetentionBoundary {
    pub plaintext_writing_retained: bool,
    pub session_payload_retained: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorSessionSnapshot {
    pub session: CanonicalProcessorSessionIdentity,
    pub paths: CanonicalProcessorReadbackPaths,
    pub status: CanonicalProcessorStatusSnapshot,
    pub artifacts: CanonicalProcessorArtifacts,
    pub proof: CanonicalProofReadback,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProcessorStatusResponse {
    pub ok: bool,
    pub canonical_path: &'static str,
    pub core_loop: &'static str,
    pub snapshot: CanonicalProcessorSessionSnapshot,
    pub legacy_retention: LegacyProcessorRetentionBoundary,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProofResponse {
    pub ok: bool,
    pub canonical_path: &'static str,
    pub core_loop: &'static str,
    pub session: CanonicalProcessorSessionIdentity,
    pub paths: CanonicalProcessorReadbackPaths,
    pub proof: CanonicalProofReadback,
}

pub fn canonical_anky_session_status_path(session_hash: &str) -> String {
    format!(
        "{}/{}",
        CANONICAL_ANKY_SESSIONS_PATH_PREFIX,
        session_hash.trim()
    )
}

pub fn canonical_anky_session_proof_path(session_hash: &str) -> String {
    format!(
        "{}/{}/proof",
        CANONICAL_ANKY_SESSIONS_PATH_PREFIX,
        session_hash.trim()
    )
}

pub fn proof_metadata_from_legacy_solana(
    session_hash: &str,
    legacy_solana_status: &str,
    legacy_solana_signature: Option<&str>,
) -> ProofOfWritingMetadata {
    let receipt = legacy_solana_signature
        .map(str::trim)
        .filter(|signature| !signature.is_empty())
        .map(|signature| ProofOfWritingReceipt {
            anchor_kind: ProofOfWritingAnchorKind::SolanaTransaction,
            anchor_id: signature.to_string(),
        });

    ProofOfWritingMetadata {
        session_hash: session_hash.trim().to_string(),
        proof_status: CanonicalProofStatus::from_legacy_solana_status(legacy_solana_status),
        receipt,
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactValidationStatus {
    Missing,
    Invalid,
    Complete,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompletedAnkyArtifactValidation {
    pub title: ArtifactValidationStatus,
    pub reflection: ArtifactValidationStatus,
    pub image: ArtifactValidationStatus,
    pub proof: ArtifactValidationStatus,
}

impl CompletedAnkyArtifactValidation {
    pub fn is_complete(&self) -> bool {
        self.title == ArtifactValidationStatus::Complete
            && self.reflection == ArtifactValidationStatus::Complete
            && self.image == ArtifactValidationStatus::Complete
            && self.proof == ArtifactValidationStatus::Complete
    }
}

pub fn qualifies_as_canonical_anky(duration_seconds: i64, word_count: i32) -> bool {
    duration_seconds >= CANONICAL_ANKY_MIN_DURATION_SECONDS
        && word_count >= CANONICAL_ANKY_MIN_WORD_COUNT
}

pub fn qualifies_as_canonical_anky_f64(duration_seconds: f64, word_count: i32) -> bool {
    duration_seconds >= CANONICAL_ANKY_MIN_DURATION_SECONDS as f64
        && word_count >= CANONICAL_ANKY_MIN_WORD_COUNT
}

pub fn legacy_sealed_write_qualifies_as_anky(duration_seconds: f64, word_count: i32) -> bool {
    duration_seconds >= CANONICAL_ANKY_MIN_DURATION_SECONDS as f64
        && word_count >= LEGACY_SEALED_WRITE_MIN_WORD_COUNT
}

pub fn normalize_title_candidate(title: &str) -> String {
    title.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn canonical_title_word_count(title: &str) -> usize {
    normalize_title_candidate(title).split_whitespace().count()
}

pub fn validate_canonical_title(title: &str) -> ArtifactValidationStatus {
    let normalized = normalize_title_candidate(title);
    if normalized.is_empty() {
        ArtifactValidationStatus::Missing
    } else if canonical_title_word_count(&normalized) == CANONICAL_TITLE_WORD_COUNT {
        ArtifactValidationStatus::Complete
    } else {
        ArtifactValidationStatus::Invalid
    }
}

pub fn validate_completed_anky_artifacts(
    title: Option<&str>,
    reflection: Option<&str>,
    image_url: Option<&str>,
    proof_metadata: &ProofOfWritingMetadata,
) -> CompletedAnkyArtifactValidation {
    CompletedAnkyArtifactValidation {
        title: title
            .map(validate_canonical_title)
            .unwrap_or(ArtifactValidationStatus::Missing),
        reflection: validate_required_text_artifact(reflection),
        image: validate_required_text_artifact(image_url),
        proof: validate_proof_metadata(proof_metadata),
    }
}

pub fn validate_required_text_artifact(value: Option<&str>) -> ArtifactValidationStatus {
    match value.map(str::trim) {
        Some(value) if !value.is_empty() => ArtifactValidationStatus::Complete,
        _ => ArtifactValidationStatus::Missing,
    }
}

pub fn validate_proof_metadata(
    proof_metadata: &ProofOfWritingMetadata,
) -> ArtifactValidationStatus {
    match proof_metadata.proof_status {
        CanonicalProofStatus::Complete => {
            if proof_metadata.session_hash.is_empty() || proof_metadata.receipt.is_none() {
                ArtifactValidationStatus::Invalid
            } else {
                ArtifactValidationStatus::Complete
            }
        }
        CanonicalProofStatus::Failed => ArtifactValidationStatus::Invalid,
        CanonicalProofStatus::Pending
        | CanonicalProofStatus::InProgress
        | CanonicalProofStatus::Skipped => ArtifactValidationStatus::Missing,
    }
}

pub fn canonical_core_loop_status(
    reflection_status: &str,
    image_status: &str,
    proof_status: CanonicalProofStatus,
    has_error: bool,
    is_done: bool,
) -> CanonicalCoreLoopStatus {
    if has_error {
        return CanonicalCoreLoopStatus::Failed;
    }
    if is_done {
        return CanonicalCoreLoopStatus::Complete;
    }
    if reflection_status == "pending"
        && image_status == "pending"
        && proof_status == CanonicalProofStatus::Pending
    {
        return CanonicalCoreLoopStatus::Accepted;
    }
    if reflection_status != "complete" {
        return CanonicalCoreLoopStatus::Reflecting;
    }
    if image_status != "complete" {
        return CanonicalCoreLoopStatus::Imaging;
    }
    CanonicalCoreLoopStatus::Proving
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_validation_requires_exactly_three_words() {
        assert_eq!(
            validate_canonical_title("sacred eight minutes"),
            ArtifactValidationStatus::Complete
        );
        assert_eq!(
            validate_canonical_title(" sacred   eight   minutes "),
            ArtifactValidationStatus::Complete
        );
        assert_eq!(
            validate_canonical_title("untitled reflection"),
            ArtifactValidationStatus::Invalid
        );
    }

    #[test]
    fn proof_metadata_requires_a_receipt_for_completion() {
        let invalid = ProofOfWritingMetadata {
            session_hash: "abc".into(),
            proof_status: CanonicalProofStatus::Complete,
            receipt: None,
        };
        assert_eq!(
            validate_proof_metadata(&invalid),
            ArtifactValidationStatus::Invalid
        );

        let valid = proof_metadata_from_legacy_solana("abc", "complete", Some("sig-123"));
        assert_eq!(
            validate_proof_metadata(&valid),
            ArtifactValidationStatus::Complete
        );
    }

    #[test]
    fn artifact_completeness_flags_invalid_title_and_missing_proof() {
        let proof = proof_metadata_from_legacy_solana("abc", "skipped", None);
        let validation = validate_completed_anky_artifacts(
            Some("untitled reflection"),
            Some("reflection"),
            Some("https://cdn.example/image.webp"),
            &proof,
        );

        assert_eq!(validation.title, ArtifactValidationStatus::Invalid);
        assert_eq!(validation.reflection, ArtifactValidationStatus::Complete);
        assert_eq!(validation.image, ArtifactValidationStatus::Complete);
        assert_eq!(validation.proof, ArtifactValidationStatus::Missing);
        assert!(!validation.is_complete());
    }

    #[test]
    fn qualification_constants_are_centralized() {
        assert!(qualifies_as_canonical_anky(480, 300));
        assert!(!qualifies_as_canonical_anky(479, 300));
        assert!(!qualifies_as_canonical_anky(480, 299));
        assert!(legacy_sealed_write_qualifies_as_anky(480.0, 50));
        assert!(!legacy_sealed_write_qualifies_as_anky(479.0, 50));
    }
}
