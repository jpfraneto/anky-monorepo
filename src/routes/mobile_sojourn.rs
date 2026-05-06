use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::{
    STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD,
};
use base64::Engine as _;
use jsonwebtoken::{encode as jwt_encode, Algorithm, EncodingKey, Header};
use mpl_core::instructions::CreateV2Builder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_sdk::{
    hash::Hash as SolanaHash,
    pubkey::Pubkey as SolanaPubkey,
    signature::{Keypair as SolanaKeypair, Signer as SolanaSigner},
    transaction::Transaction as SolanaTransaction,
};
use sqlx::Row;
use std::str::FromStr;

const SOJOURN_NUMBER: u8 = 9;
const SOJOURN_STARTS_AT_UTC: &str = "2026-03-03T00:00:00.000Z";
const SOJOURN_DAY_LENGTH_SECONDS: u32 = 86_400;
const DEFAULT_SOLANA_CLUSTER: &str = "devnet";
const DEFAULT_SOLANA_RPC_URL: &str = "https://api.devnet.solana.com";
const DEFAULT_MAINNET_SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_CORE_PROGRAM_ID: &str = "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d";
const DEFAULT_CORE_COLLECTION: &str = "F9UZwmeRTBwfVVJnbXYXUjxuQGYMYDEG28eXJgyF9V5u";
const DEFAULT_SEAL_PROGRAM_ID: &str = "4GjZaHbyyeVEjeYjm2q7vVdnNhMPnNMx8oeRwEBZDsMX";
const DEFAULT_PROOF_VERIFIER_AUTHORITY: &str = "FgFFj9ZCeEG7dYKaWqtTm3q6apjqBxvDq5QVjkajpCGP";
const DEFAULT_COLLECTION_URI: &str = "https://anky.app/devnet/metadata/sojourn-9-looms.json";
const DEFAULT_MAINNET_COLLECTION_URI: &str =
    "https://anky.app/mainnet/metadata/sojourn-9-looms.json";
const DEFAULT_LOOM_METADATA_BASE_URL: &str = "https://anky.app/devnet/metadata/looms";
const DEFAULT_MAINNET_LOOM_METADATA_BASE_URL: &str = "https://anky.app/mainnet/metadata/looms";
const DEFAULT_SOJOURN_9_PROGRAM_ID: &str = "2VfB7nvV2SZuCpK2DurRgJLfw57TCt2g9VJXACo5h8aK";
const DEFAULT_INITIAL_MOBILE_CREDITS: u32 = 8;
const DEV_RECEIPT_SECRET: &str = "dev-mobile-receipt-secret";
const MAX_LOOM_INDEX: u32 = 3_456;
const MAX_HELIUS_WEBHOOK_PAYLOAD_BYTES: usize = 2_000_000;
const HELIUS_WEBHOOK_SOURCE: &str = "helius_enhanced_webhook";
const VERIFIED_SEAL_SEED: &[u8] = b"verified_seal";
const PLACEHOLDER_IMAGE_PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/config", get(get_config))
        .route("/api/v1/credits/balance", get(get_credit_balance))
        .route("/api/v1/credits/checkout", post(create_checkout_session))
        // Legacy direct-IAP verifier. The mobile app now uses RevenueCat CREDITS
        // and should not call this route.
        .route(
            "/api/v1/credits/native-purchase/verify",
            post(verify_native_credit_purchase),
        )
        .route("/api/v1/processing/tickets", post(create_processing_ticket))
        .route("/api/v1/processing/run", post(run_processing))
        .route("/api/v1/seals", get(lookup_seals))
        .route("/api/mobile/solana/config", get(get_mobile_solana_config))
        .route("/api/mobile/credits", get(get_mobile_credit_balance))
        .route("/api/mobile/credits/spend", post(spend_mobile_credits))
        .route(
            "/api/mobile/looms/mint-authorizations",
            post(create_mobile_mint_authorization),
        )
        .route(
            "/api/mobile/looms/prepare-mint",
            post(prepare_mobile_loom_mint),
        )
        .route("/api/mobile/looms/record", post(record_mobile_loom_mint))
        .route("/api/mobile/looms", get(lookup_mobile_looms))
        .route("/api/mobile/threads", post(create_mobile_thread))
        .route("/api/mobile/reflections", post(create_mobile_reflection))
        .route(
            "/api/mobile/reflections/{job_id}",
            get(get_mobile_reflection),
        )
        .route("/api/mobile/seals", get(lookup_mobile_seals))
        .route("/api/mobile/seals/score", get(get_mobile_seal_score))
        .route("/api/mobile/seals/record", post(record_mobile_seal))
        .route(
            "/api/mobile/seals/verified/record",
            post(record_mobile_verified_seal),
        )
        .route(
            "/api/helius/anky-seal",
            post(record_helius_anky_seal_webhook),
        )
}

pub async fn get_config() -> Json<AppConfigResponse> {
    Json(AppConfigResponse {
        sojourn: SojournConfig {
            number: SOJOURN_NUMBER,
            starts_at_utc: SOJOURN_STARTS_AT_UTC.to_string(),
            day_length_seconds: SOJOURN_DAY_LENGTH_SECONDS,
        },
        solana: SolanaConfig {
            cluster: solana_cluster(),
            anky_program_id: Some(sojourn_9_program_id()),
            rpc_url: Some(public_solana_rpc_url()),
            core_program_id: Some(core_program_id()),
            core_collection: Some(core_collection()),
            seal_program_id: Some(seal_program_id()),
            proof_verifier_authority: Some(proof_verifier_authority()),
            collection_uri: Some(collection_uri()),
            loom_metadata_base_url: Some(loom_metadata_base_url()),
            seal_verification: Some(seal_verification_label()),
        },
        processing: ProcessingConfig {
            public_key: env_nonempty("ANKY_PROCESSING_PUBLIC_KEY"),
            dev_plaintext_processing_allowed: dev_plaintext_processing_allowed(),
        },
    })
}

pub async fn get_credit_balance() -> Json<CreditBalanceResponse> {
    Json(CreditBalanceResponse {
        credits_remaining: dev_credit_balance(),
    })
}

pub async fn create_checkout_session(
    Json(req): Json<CreateCheckoutRequest>,
) -> Result<Json<CreateCheckoutResponse>, AppError> {
    if req.package_id.trim().is_empty() {
        return Err(AppError::BadRequest("packageId is required".into()));
    }

    if let Some(base_url) = env_nonempty("ANKY_CREDITS_CHECKOUT_BASE_URL") {
        return Ok(Json(CreateCheckoutResponse {
            checkout_url: format!(
                "{}/{}",
                base_url.trim_end_matches('/'),
                req.package_id.trim()
            ),
        }));
    }

    Err(AppError::Unavailable(
        "credit checkout is not configured on this backend".into(),
    ))
}

pub async fn verify_native_credit_purchase(
    State(state): State<AppState>,
    Json(req): Json<NativeCreditPurchaseVerifyRequest>,
) -> Result<Json<NativeCreditPurchaseVerifyResponse>, AppError> {
    let identity_id = req.identity_id.trim();

    if identity_id.is_empty() {
        return Err(AppError::BadRequest("identityId is required".into()));
    }

    let package = native_credit_package(&req.package_id)
        .ok_or_else(|| AppError::BadRequest("unknown credit package".into()))?;

    validate_native_credit_purchase_request(&req, &package)?;
    verify_native_store_purchase(&req).await?;

    let (account, credits_added, duplicate) =
        grant_native_mobile_credits(&state.db, identity_id, &req, &package).await?;

    Ok(Json(NativeCreditPurchaseVerifyResponse {
        account,
        credits_added,
        duplicate,
        ok: true,
    }))
}

pub async fn create_processing_ticket(
    Json(req): Json<CreateProcessingTicketRequest>,
) -> Result<Json<CreateProcessingTicketResponse>, AppError> {
    validate_processing_ticket_request(&req)?;

    let secret = receipt_secret()?;
    let cost = req.processing_type.credit_cost();
    let current_balance = dev_credit_balance();
    let remaining = current_balance.saturating_sub(cost);
    let issued_at = chrono::Utc::now().timestamp_millis();
    let expires_at = issued_at + 15 * 60 * 1000;
    let ticket_id = uuid::Uuid::new_v4().to_string();
    let nonce = hex::encode(Sha256::digest(format!(
        "{}:{}:{}",
        ticket_id,
        req.processing_type.as_str(),
        req.session_hashes.join(",")
    )));
    let signature = sign_receipt_fields(
        &secret,
        &ticket_id,
        req.processing_type,
        cost,
        remaining,
        issued_at,
        expires_at,
        &nonce,
    );

    Ok(Json(CreateProcessingTicketResponse {
        receipt: CreditReceipt {
            receipt_version: 1,
            ticket_id,
            processing_type: req.processing_type,
            credits_spent: cost,
            credits_remaining: remaining,
            issued_at,
            expires_at,
            nonce,
            signature,
        },
    }))
}

pub async fn run_processing(
    Json(req): Json<RunProcessingRequest>,
) -> Result<Json<RunProcessingResponse>, AppError> {
    validate_receipt(&req.receipt)?;

    match req.encryption_scheme {
        Some(EncryptionScheme::DevPlaintext) => {
            if !dev_plaintext_processing_allowed() {
                return Err(AppError::Unavailable(
                    "dev plaintext carpet processing is disabled".into(),
                ));
            }
        }
        Some(EncryptionScheme::X25519V1) => {
            return Err(AppError::Unavailable(
                "x25519_v1 carpet decryption is not implemented on this backend yet".into(),
            ));
        }
        None => {
            return Err(AppError::BadRequest("encryptionScheme is required".into()));
        }
    }

    let carpet: AnkyCarpet = serde_json::from_str(&req.encrypted_carpet)
        .map_err(|_| AppError::BadRequest("encryptedCarpet is not a valid dev carpet".into()))?;
    validate_carpet(&carpet)?;

    if carpet.purpose != req.receipt.processing_type {
        return Err(AppError::BadRequest(
            "receipt processingType does not match carpet purpose".into(),
        ));
    }

    let carpet_hash = hash_hex(req.encrypted_carpet.as_bytes());
    let artifacts = build_dev_artifacts(&carpet, &carpet_hash)?;

    Ok(Json(RunProcessingResponse {
        processing_type: carpet.purpose,
        artifacts,
    }))
}

pub async fn lookup_seals(
    State(state): State<AppState>,
    Query(query): Query<SealLookupQuery>,
) -> Result<Json<SealLookupResponse>, AppError> {
    validate_seal_lookup_query(&query)?;
    let seals = query_seal_receipts(&state.db, &query).await?;

    Ok(Json(SealLookupResponse { seals }))
}

pub async fn get_mobile_solana_config() -> Json<MobileSolanaConfigResponse> {
    Json(mobile_solana_config())
}

pub async fn get_mobile_credit_balance(
    State(state): State<AppState>,
    Query(query): Query<MobileCreditQuery>,
) -> Result<Json<MobileCreditResponse>, AppError> {
    let identity_id = validate_identity_id(&query.identity_id)?;
    let account = ensure_mobile_credit_account(&state.db, &identity_id).await?;

    Ok(Json(MobileCreditResponse {
        account,
        initial_credits: initial_mobile_credits(),
    }))
}

pub async fn spend_mobile_credits(
    State(state): State<AppState>,
    Json(req): Json<MobileSpendCreditsRequest>,
) -> Result<Json<MobileSpendCreditsResponse>, AppError> {
    let identity_id = validate_identity_id(&req.identity_id)?;
    if req.amount == 0 || req.amount > 1_000 {
        return Err(AppError::BadRequest(
            "amount must be between 1 and 1000".into(),
        ));
    }
    let reason = validate_short_text("reason", &req.reason, 96)?;

    ensure_mobile_credit_account(&state.db, &identity_id).await?;
    let account = debit_mobile_credits(
        &state.db,
        &identity_id,
        req.amount,
        &reason,
        req.related_id.as_deref(),
        req.metadata.unwrap_or_else(|| json!({})),
    )
    .await?;

    Ok(Json(MobileSpendCreditsResponse {
        account,
        credits_spent: req.amount,
    }))
}

pub async fn create_mobile_mint_authorization(
    State(state): State<AppState>,
    Json(req): Json<MobileMintAuthorizationRequest>,
) -> Result<Json<MobileMintAuthorizationResponse>, AppError> {
    let wallet = validate_public_key("wallet", &req.wallet)?;
    let payer = match req.payer.as_deref() {
        Some(payer) => validate_public_key("payer", payer)?,
        None => wallet.clone(),
    };
    let collection = req.collection.unwrap_or_else(core_collection);
    validate_expected_collection(&collection)?;
    validate_loom_index(req.loom_index)?;

    let mode = if req.invite_code.is_some() {
        "invite_code"
    } else {
        "self_funded"
    };
    let invite_code_hash = req.invite_code.as_deref().map(hash_invite_code);
    let invite_allowed = req
        .invite_code
        .as_deref()
        .map(invite_code_is_allowed)
        .unwrap_or(true);
    let allowed = req.invite_code.is_none() || invite_allowed;
    let reason = if allowed {
        None
    } else {
        Some("invite code is not authorized for devnet Loom minting".to_string())
    };
    let sponsor = allowed && req.invite_code.is_some() && env_flag("ANKY_DEV_SPONSOR_INVITE_MINTS");
    let sponsor_payer = if sponsor {
        env_nonempty("ANKY_SPONSOR_PAYER")
    } else {
        None
    };
    let authorization_id = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(15);
    let signature = sign_mint_authorization(
        &authorization_id,
        &wallet,
        &payer,
        &collection,
        req.loom_index,
        mode,
        allowed,
        expires_at.timestamp_millis(),
    );

    sqlx::query(
        "INSERT INTO mobile_mint_authorizations
         (id, network, wallet, payer, core_collection, loom_index, mode, invite_code_hash, allowed, sponsor, sponsor_payer, reason, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&authorization_id)
    .bind(solana_cluster())
    .bind(&wallet)
    .bind(&payer)
    .bind(&collection)
    .bind(req.loom_index as i32)
    .bind(mode)
    .bind(invite_code_hash)
    .bind(allowed)
    .bind(sponsor)
    .bind(sponsor_payer.as_deref())
    .bind(&reason)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    Ok(Json(MobileMintAuthorizationResponse {
        allowed,
        authorization_id,
        collection,
        expires_at: expires_at.to_rfc3339(),
        loom_index: req.loom_index,
        mode: mode.to_string(),
        owner: wallet,
        payer,
        reason,
        signature,
        sponsor,
        sponsor_payer,
    }))
}

pub async fn prepare_mobile_loom_mint(
    State(state): State<AppState>,
    Json(req): Json<PrepareMobileLoomMintRequest>,
) -> Result<Json<PrepareMobileLoomMintResponse>, AppError> {
    let wallet = validate_public_key("wallet", &req.wallet)?;
    let payer = match req.payer.as_deref() {
        Some(payer) => validate_public_key("payer", payer)?,
        None => wallet.clone(),
    };
    let collection = req.collection.unwrap_or_else(core_collection);
    validate_expected_collection(&collection)?;
    validate_loom_index(req.loom_index)?;

    let authorization = lookup_mobile_mint_authorization(
        &state.db,
        &req.authorization_id,
        &wallet,
        &payer,
        &collection,
        req.loom_index,
    )
    .await?;

    let loom_number = format_loom_number(req.loom_index);
    let name = format!("Anky Sojourn 9 Loom #{}", loom_number);
    let uri = req
        .metadata_uri
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("{}/{}.json", loom_metadata_base_url(), loom_number));
    let prepared =
        build_core_loom_mint_transaction(&wallet, &payer, &collection, &name, &uri).await?;

    Ok(Json(PrepareMobileLoomMintResponse {
        asset: prepared.asset,
        authorization_id: authorization.authorization_id,
        blockhash: prepared.blockhash,
        collection,
        collection_authority: prepared.collection_authority,
        last_valid_block_height: prepared.last_valid_block_height,
        loom_index: req.loom_index,
        mode: authorization.mode,
        name,
        owner: wallet,
        payer,
        transaction_base64: prepared.transaction_base64,
        uri,
    }))
}

pub async fn record_mobile_loom_mint(
    State(state): State<AppState>,
    Json(req): Json<RecordMobileLoomMintRequest>,
) -> Result<Json<RecordMobileLoomMintResponse>, AppError> {
    let wallet = validate_public_key("wallet", &req.wallet)?;
    let loom_asset = validate_public_key("loomAsset", &req.loom_asset)?;
    let core_collection = validate_public_key("coreCollection", &req.core_collection)?;
    validate_expected_collection(&core_collection)?;
    let signature = validate_signature(&req.signature)?;
    if let Some(loom_index) = req.loom_index {
        validate_loom_index(loom_index)?;
    }
    let status = validate_status(req.status.as_deref())?;

    let row = sqlx::query(
        "INSERT INTO mobile_loom_mints
         (id, network, wallet, loom_asset, core_collection, signature, loom_index, mint_mode, metadata_uri, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (signature) DO UPDATE
         SET status = EXCLUDED.status,
             metadata_uri = COALESCE(EXCLUDED.metadata_uri, mobile_loom_mints.metadata_uri)
         RETURNING id, network, wallet, loom_asset, core_collection, signature, loom_index, mint_mode, metadata_uri, status, created_at",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(solana_cluster())
    .bind(&wallet)
    .bind(&loom_asset)
    .bind(&core_collection)
    .bind(&signature)
    .bind(req.loom_index.map(|value| value as i32))
    .bind(req.mint_mode.as_deref())
    .bind(req.metadata_uri.as_deref())
    .bind(&status)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(RecordMobileLoomMintResponse {
        recorded: true,
        loom: mobile_loom_mint_from_row(&row)?,
    }))
}

pub async fn lookup_mobile_looms(
    State(state): State<AppState>,
    Query(query): Query<MobileLoomLookupQuery>,
) -> Result<Json<MobileLoomLookupResponse>, AppError> {
    let wallet = validate_public_key("wallet", &query.wallet)?;
    let rows = sqlx::query(
        "SELECT id, network, wallet, loom_asset, core_collection, signature, loom_index, mint_mode, metadata_uri, status, created_at
         FROM mobile_loom_mints
         WHERE wallet = $1
         ORDER BY created_at DESC
         LIMIT 100",
    )
    .bind(wallet)
    .fetch_all(&state.db)
    .await?;

    let looms = rows
        .iter()
        .map(mobile_loom_mint_from_row)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(MobileLoomLookupResponse { looms }))
}

pub async fn create_mobile_thread(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<MobileThreadResponse>, MobileThreadError> {
    let req = validate_mobile_thread_payload(payload)?;
    let message_count = req.messages.len();

    tracing::info!(
        session_hash = %req.session_hash,
        mode = %req.mode.as_str(),
        message_count,
        "mobile thread request received"
    );

    if mobile_thread_needs_immediate_safety_response(&req) {
        tracing::info!(
            session_hash = %req.session_hash,
            mode = %req.mode.as_str(),
            message_count,
            "mobile thread safety response returned"
        );
        return Ok(Json(
            mobile_thread_response(mobile_thread_safety_response()),
        ));
    }

    let system = build_mobile_thread_system_prompt(&req);
    let provider_messages = build_mobile_thread_provider_messages(&req);
    let model_text = crate::services::claude::chat_with_system_best(
        &state.config,
        &system,
        &provider_messages,
        420,
    )
    .await
    .map_err(|_error| {
        tracing::warn!(
            session_hash = %req.session_hash,
            mode = %req.mode.as_str(),
            message_count,
            "mobile thread provider failed"
        );
        MobileThreadError::ThreadUnavailable
    })?;
    let content = normalize_mobile_thread_reply(&model_text).map_err(|_error| {
        tracing::warn!(
            session_hash = %req.session_hash,
            mode = %req.mode.as_str(),
            message_count,
            "mobile thread provider returned unusable text"
        );
        MobileThreadError::ThreadUnavailable
    })?;

    tracing::info!(
        session_hash = %req.session_hash,
        mode = %req.mode.as_str(),
        message_count,
        "mobile thread response generated"
    );

    Ok(Json(mobile_thread_response(content)))
}

pub async fn create_mobile_reflection(
    State(state): State<AppState>,
    Json(req): Json<MobileReflectionRequest>,
) -> Result<Json<MobileReflectionResponse>, AppError> {
    let MobileReflectionRequest {
        identity_id,
        session_hash,
        anky,
        processing_type,
    } = req;
    let identity_id = validate_identity_id(&identity_id)?;
    let session_hash = normalize_hash(&session_hash)?;
    let processing_type = processing_type.unwrap_or(ProcessingType::Reflection);

    if !matches!(
        processing_type,
        ProcessingType::Reflection | ProcessingType::FullAnky
    ) {
        return Err(AppError::BadRequest(
            "/api/mobile/reflections only supports processingType=reflection or full_anky".into(),
        ));
    }

    // This endpoint is an explicit opt-in plaintext processing path. The raw
    // `.anky` bytes are validated, reconstructed for reflection, and dropped
    // before any database writes; persisted rows store only derived metadata.
    let computed_hash = hash_hex(anky.as_bytes());
    if computed_hash != session_hash {
        return Err(AppError::BadRequest(
            ".anky bytes do not match sessionHash".into(),
        ));
    }
    validate_closed_anky(&anky)?;
    let anky_byte_length = anky.as_bytes().len();
    let writing_text = reconstruct_closed_anky_text(&anky)?;
    drop(anky);

    ensure_mobile_credit_account(&state.db, &identity_id).await?;

    let artifacts =
        build_mobile_reflection_artifacts(&state, processing_type, &session_hash, &writing_text)
            .await?;
    let cost = processing_type.credit_cost();
    let account = debit_mobile_credits(
        &state.db,
        &identity_id,
        cost,
        processing_type.as_str(),
        Some(&session_hash),
        json!({ "sessionHash": session_hash, "processingType": processing_type.as_str() }),
    )
    .await?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let request_json = json!({
        "sessionHash": session_hash,
        "processingType": processing_type.as_str(),
        "ankyByteLength": anky_byte_length,
        "entryCount": 1,
        "plaintextReceivedByBackend": true
    });
    let result_json = json!({
        "processingType": processing_type.as_str(),
        "artifacts": artifacts.clone()
    });

    let row = sqlx::query(
        "INSERT INTO mobile_reflection_jobs
         (id, identity_id, session_hash, processing_type, status, credits_spent, request_json, result_json)
         VALUES ($1, $2, $3, $4, 'complete', $5, $6, $7)
         RETURNING id, identity_id, session_hash, processing_type, status, credits_spent, request_json, result_json, error, created_at, updated_at",
    )
    .bind(&job_id)
    .bind(&identity_id)
    .bind(&session_hash)
    .bind(processing_type.as_str())
    .bind(cost as i32)
    .bind(request_json.to_string())
    .bind(result_json.to_string())
    .fetch_one(&state.db)
    .await?;

    let job = mobile_reflection_job_from_row(&row)?;

    Ok(Json(MobileReflectionResponse {
        artifacts,
        credits_remaining: account.credits_remaining,
        credits_spent: cost,
        job,
    }))
}

pub async fn get_mobile_reflection(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<MobileReflectionJobResponse>, AppError> {
    let row = sqlx::query(
        "SELECT id, identity_id, session_hash, processing_type, status, credits_spent, request_json, result_json, error, created_at, updated_at
         FROM mobile_reflection_jobs
         WHERE id = $1",
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("reflection job not found".into()))?;

    Ok(Json(MobileReflectionJobResponse {
        job: mobile_reflection_job_from_row(&row)?,
    }))
}

pub async fn lookup_mobile_seals(
    State(state): State<AppState>,
    Query(query): Query<SealLookupQuery>,
) -> Result<Json<SealLookupResponse>, AppError> {
    validate_seal_lookup_query(&query)?;
    let seals = query_seal_receipts(&state.db, &query).await?;

    Ok(Json(SealLookupResponse { seals }))
}

pub async fn get_mobile_seal_score(
    State(state): State<AppState>,
    Query(query): Query<SealScoreQuery>,
) -> Result<Json<MobileSealScoreResponse>, AppError> {
    let wallet = validate_public_key("wallet", &query.wallet)?;
    let network = solana_cluster();
    let proof_verifier = proof_verifier_authority();

    let sealed_rows = sqlx::query(
        "SELECT DISTINCT utc_day
         FROM mobile_seal_receipts
         WHERE network = $1
           AND wallet = $2
           AND utc_day IS NOT NULL
           AND status = 'finalized'
         ORDER BY utc_day ASC",
    )
    .bind(&network)
    .bind(&wallet)
    .fetch_all(&state.db)
    .await?;

    let verified_rows = sqlx::query(
        "SELECT DISTINCT verified.utc_day
         FROM mobile_verified_seal_receipts verified
         JOIN mobile_seal_receipts seal
           ON seal.network = verified.network
          AND seal.wallet = verified.wallet
          AND seal.session_hash = verified.session_hash
          AND seal.utc_day = verified.utc_day
         WHERE verified.network = $1
           AND verified.wallet = $2
           AND verified.verifier = $3
           AND verified.protocol_version = 1
           AND verified.utc_day IS NOT NULL
           AND verified.status = 'finalized'
           AND seal.status = 'finalized'
         ORDER BY verified.utc_day ASC",
    )
    .bind(&network)
    .bind(&wallet)
    .bind(&proof_verifier)
    .fetch_all(&state.db)
    .await?;

    let sealed_days = sealed_rows
        .iter()
        .map(|row| row.try_get("utc_day"))
        .collect::<Result<Vec<i64>, _>>()?;
    let verified_days = verified_rows
        .iter()
        .map(|row| row.try_get("utc_day"))
        .collect::<Result<Vec<i64>, _>>()?;

    Ok(Json(build_mobile_seal_score(
        wallet,
        network,
        proof_verifier,
        sealed_days,
        verified_days,
    )))
}

pub async fn record_mobile_seal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RecordMobileSealRequest>,
) -> Result<Json<RecordMobileSealResponse>, AppError> {
    let wallet = validate_public_key("wallet", &req.wallet)?;
    let loom_asset = validate_public_key("loomAsset", &req.loom_asset)?;
    let core_collection = validate_public_key("coreCollection", &req.core_collection)?;
    validate_expected_collection(&core_collection)?;
    let session_hash = normalize_hash(&req.session_hash)?;
    let signature = validate_signature(&req.signature)?;
    let status = validate_status(req.status.as_deref())?;
    require_finalized_seal_record_secret(&status, &headers)?;
    let can_update_finalized_receipt = indexer_write_secret_matches_config(&headers);
    let utc_day = validate_optional_utc_day(req.utc_day)?;

    let row = sqlx::query(
        "INSERT INTO mobile_seal_receipts
         (id, network, wallet, loom_asset, core_collection, session_hash, signature, utc_day, slot, block_time, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (network, wallet, session_hash) DO UPDATE
         SET utc_day = COALESCE(EXCLUDED.utc_day, mobile_seal_receipts.utc_day),
             slot = COALESCE(EXCLUDED.slot, mobile_seal_receipts.slot),
             block_time = COALESCE(EXCLUDED.block_time, mobile_seal_receipts.block_time),
             signature = EXCLUDED.signature,
             status = EXCLUDED.status
         WHERE mobile_seal_receipts.status <> 'finalized'
            OR ($12 AND EXCLUDED.status = 'finalized')
         RETURNING id, network, wallet, loom_asset, core_collection, session_hash, signature, utc_day, slot, block_time, status, created_at",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(solana_cluster())
    .bind(&wallet)
    .bind(&loom_asset)
    .bind(&core_collection)
    .bind(&session_hash)
    .bind(&signature)
    .bind(utc_day)
    .bind(req.slot.map(|slot| slot as i64))
    .bind(req.block_time)
    .bind(&status)
    .bind(can_update_finalized_receipt)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        AppError::BadRequest(
            "finalized seal metadata is immutable without a finalized indexer/operator receipt"
                .into(),
        )
    })?;

    Ok(Json(RecordMobileSealResponse {
        recorded: true,
        seal: loom_seal_from_row(&row)?,
    }))
}

pub async fn record_mobile_verified_seal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RecordMobileVerifiedSealRequest>,
) -> Result<Json<RecordMobileVerifiedSealResponse>, AppError> {
    require_verified_seal_record_secret(&headers)?;

    let wallet = validate_public_key("wallet", &req.wallet)?;
    let session_hash = normalize_hash(&req.session_hash)?;
    let proof_hash = normalize_hash(&req.proof_hash)?;
    let verifier = validate_public_key("verifier", &req.verifier)?;
    validate_expected_proof_verifier(&verifier)?;
    let signature = validate_signature(&req.signature)?;
    let status = validate_verified_seal_status(req.status.as_deref())?;
    let requested_utc_day = validate_optional_utc_day(req.utc_day)?;

    if req.protocol_version != 1 {
        return Err(AppError::BadRequest(
            "protocolVersion must be 1 for Sojourn 9 SP1 receipts".into(),
        ));
    }

    let seal_row = sqlx::query(
        "SELECT utc_day, status FROM mobile_seal_receipts
         WHERE network = $1 AND wallet = $2 AND session_hash = $3
         LIMIT 1",
    )
    .bind(solana_cluster())
    .bind(&wallet)
    .bind(&session_hash)
    .fetch_optional(&state.db)
    .await?;

    let seal_row = seal_row.ok_or_else(|| {
        AppError::BadRequest(
            "cannot record a verified receipt before the matching seal receipt is known".into(),
        )
    })?;
    let seal_utc_day: Option<i64> = seal_row.try_get("utc_day")?;
    let seal_status: String = seal_row.try_get("status")?;
    require_landed_seal_receipt_status(&seal_status)?;
    let utc_day = resolve_verified_utc_day(requested_utc_day, seal_utc_day)?;

    if require_verified_seal_chain_proof() {
        verify_verified_seal_account_on_chain(
            &wallet,
            &session_hash,
            utc_day,
            &proof_hash,
            &verifier,
            req.protocol_version,
        )
        .await?;
    }

    let row = sqlx::query(
        "INSERT INTO mobile_verified_seal_receipts
         (id, network, wallet, session_hash, proof_hash, verifier, protocol_version, utc_day, signature, slot, block_time, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         ON CONFLICT (network, wallet, session_hash) DO UPDATE
         SET slot = COALESCE(EXCLUDED.slot, mobile_verified_seal_receipts.slot),
             block_time = COALESCE(EXCLUDED.block_time, mobile_verified_seal_receipts.block_time),
             status = EXCLUDED.status
         WHERE mobile_verified_seal_receipts.proof_hash = EXCLUDED.proof_hash
           AND mobile_verified_seal_receipts.verifier = EXCLUDED.verifier
           AND mobile_verified_seal_receipts.protocol_version = EXCLUDED.protocol_version
           AND mobile_verified_seal_receipts.utc_day IS NOT DISTINCT FROM EXCLUDED.utc_day
           AND mobile_verified_seal_receipts.signature = EXCLUDED.signature
         RETURNING id, network, wallet, session_hash, proof_hash, verifier, protocol_version, utc_day, signature, slot, block_time, status, created_at",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(solana_cluster())
    .bind(&wallet)
    .bind(&session_hash)
    .bind(&proof_hash)
    .bind(&verifier)
    .bind(req.protocol_version as i32)
    .bind(utc_day)
    .bind(&signature)
    .bind(req.slot.map(|slot| slot as i64))
    .bind(req.block_time)
    .bind(&status)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        AppError::BadRequest(
            "verified seal metadata conflicts with an existing immutable receipt".into(),
        )
    })?;

    let lookup = SealLookupQuery {
        wallet: Some(wallet.clone()),
        loom_id: None,
        session_hash: None,
    };
    let seal = query_seal_receipts(&state.db, &lookup)
        .await?
        .into_iter()
        .find(|seal| seal.session_hash == session_hash)
        .ok_or_else(|| AppError::NotFound("matching seal receipt not found".into()))?;

    Ok(Json(RecordMobileVerifiedSealResponse {
        recorded: true,
        seal,
        verified_seal: mobile_verified_seal_from_row(&row)?,
    }))
}

pub async fn record_helius_anky_seal_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<Json<RecordHeliusAnkySealWebhookResponse>, AppError> {
    require_verified_seal_record_secret(&headers)?;
    validate_public_webhook_payload(&payload)?;
    let payload_json = serde_json::to_string(&payload)?;
    if payload_json.len() > MAX_HELIUS_WEBHOOK_PAYLOAD_BYTES {
        return Err(AppError::BadRequest(
            "Helius webhook payload is too large".into(),
        ));
    }
    let payload_hash = hash_hex(payload_json.as_bytes());
    let signatures = collect_public_webhook_signatures(&payload);
    let signature = signatures.first().cloned();
    let event_count = count_helius_webhook_items(&payload);

    let row = if signature.is_some() {
        sqlx::query(
            "WITH signature_existing AS (
                 UPDATE mobile_helius_webhook_events
                 SET event_count = GREATEST($6, event_count),
                     payload_json = $7
                 WHERE network = $2 AND signature = $5
                 RETURNING id, network, source, payload_hash, signature, event_count, created_at
             ),
             hash_existing AS (
                 UPDATE mobile_helius_webhook_events
                 SET signature = COALESCE(signature, $5),
                     event_count = GREATEST($6, event_count),
                     payload_json = $7
                 WHERE network = $2
                   AND payload_hash = $4
                   AND NOT EXISTS (SELECT 1 FROM signature_existing)
                 RETURNING id, network, source, payload_hash, signature, event_count, created_at
             ),
             inserted AS (
                 INSERT INTO mobile_helius_webhook_events
                     (id, network, source, payload_hash, signature, event_count, payload_json)
                 SELECT $1, $2, $3, $4, $5, $6, $7
                 WHERE NOT EXISTS (SELECT 1 FROM signature_existing)
                   AND NOT EXISTS (SELECT 1 FROM hash_existing)
                 ON CONFLICT (network, signature) WHERE signature IS NOT NULL DO UPDATE
                 SET event_count = GREATEST(EXCLUDED.event_count, mobile_helius_webhook_events.event_count),
                     payload_json = EXCLUDED.payload_json
                 RETURNING id, network, source, payload_hash, signature, event_count, created_at
             )
             SELECT id, network, source, payload_hash, signature, event_count, created_at FROM signature_existing
             UNION ALL
             SELECT id, network, source, payload_hash, signature, event_count, created_at FROM hash_existing
             UNION ALL
             SELECT id, network, source, payload_hash, signature, event_count, created_at FROM inserted
             LIMIT 1",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(solana_cluster())
        .bind(HELIUS_WEBHOOK_SOURCE)
        .bind(&payload_hash)
        .bind(&signature)
        .bind(event_count as i32)
        .bind(&payload_json)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query(
            "INSERT INTO mobile_helius_webhook_events
             (id, network, source, payload_hash, signature, event_count, payload_json)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (network, payload_hash) DO UPDATE
             SET event_count = EXCLUDED.event_count,
                 payload_json = EXCLUDED.payload_json
             RETURNING id, network, source, payload_hash, signature, event_count, created_at",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(solana_cluster())
        .bind(HELIUS_WEBHOOK_SOURCE)
        .bind(&payload_hash)
        .bind(&signature)
        .bind(event_count as i32)
        .bind(&payload_json)
        .fetch_one(&state.db)
        .await?
    };

    Ok(Json(RecordHeliusAnkySealWebhookResponse {
        recorded: true,
        event: helius_webhook_event_from_row(&row)?,
    }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigResponse {
    sojourn: SojournConfig,
    solana: SolanaConfig,
    processing: ProcessingConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SojournConfig {
    number: u8,
    starts_at_utc: String,
    day_length_seconds: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SolanaConfig {
    cluster: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    anky_program_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rpc_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    core_program_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    core_collection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seal_program_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_verifier_authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    collection_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    loom_metadata_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seal_verification: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    public_key: Option<String>,
    dev_plaintext_processing_allowed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditBalanceResponse {
    credits_remaining: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCheckoutRequest {
    package_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCheckoutResponse {
    checkout_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessingTicketRequest {
    processing_type: ProcessingType,
    estimated_entry_count: usize,
    session_hashes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessingTicketResponse {
    receipt: CreditReceipt,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingType {
    Reflection,
    Image,
    FullAnky,
    DeepMirror,
    FullSojournArchive,
}

impl ProcessingType {
    fn as_str(self) -> &'static str {
        match self {
            ProcessingType::Reflection => "reflection",
            ProcessingType::Image => "image",
            ProcessingType::FullAnky => "full_anky",
            ProcessingType::DeepMirror => "deep_mirror",
            ProcessingType::FullSojournArchive => "full_sojourn_archive",
        }
    }

    fn credit_cost(self) -> u32 {
        match self {
            ProcessingType::Reflection => 1,
            ProcessingType::Image => 3,
            ProcessingType::FullAnky => 5,
            ProcessingType::DeepMirror => 8,
            ProcessingType::FullSojournArchive => 88,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditReceipt {
    receipt_version: u8,
    ticket_id: String,
    processing_type: ProcessingType,
    credits_spent: u32,
    credits_remaining: u32,
    issued_at: i64,
    expires_at: i64,
    nonce: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProcessingRequest {
    receipt: CreditReceipt,
    encrypted_carpet: String,
    encryption_scheme: Option<EncryptionScheme>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionScheme {
    DevPlaintext,
    #[serde(rename = "x25519_v1")]
    X25519V1,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProcessingResponse {
    processing_type: ProcessingType,
    artifacts: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnkyCarpet {
    carpet_version: u8,
    purpose: ProcessingType,
    created_at: i64,
    entries: Vec<CarpetEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CarpetEntry {
    session_hash: String,
    anky: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealLookupQuery {
    wallet: Option<String>,
    #[serde(alias = "loom_id")]
    loom_id: Option<String>,
    #[serde(alias = "session_hash")]
    session_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealScoreQuery {
    wallet: String,
}

#[derive(Debug, Serialize)]
pub struct SealLookupResponse {
    seals: Vec<LoomSeal>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSealScoreResponse {
    wallet: String,
    network: String,
    proof_verifier_authority: String,
    unique_seal_days: u32,
    verified_seal_days: u32,
    streak_bonus: u32,
    score: u32,
    sealed_days: Vec<i64>,
    verified_days: Vec<i64>,
    finalized_only: bool,
    formula: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoomSeal {
    tx_signature: String,
    writer: String,
    loom_id: String,
    session_hash: String,
    network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    utc_day: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_tx_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_verifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_protocol_version: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_utc_day: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_slot: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_block_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_created_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSolanaConfigResponse {
    cluster: String,
    network: String,
    rpc_url: String,
    core_program_id: String,
    core_collection: String,
    seal_program_id: String,
    proof_verifier_authority: String,
    collection_uri: String,
    loom_metadata_base_url: String,
    seal_verification: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileCreditQuery {
    identity_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileCreditAccount {
    identity_id: String,
    credits_remaining: u32,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileCreditResponse {
    account: MobileCreditAccount,
    initial_credits: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NativePurchasePlatform {
    Ios,
    Android,
}

impl NativePurchasePlatform {
    fn as_str(self) -> &'static str {
        match self {
            NativePurchasePlatform::Ios => "ios",
            NativePurchasePlatform::Android => "android",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCreditPurchaseVerifyRequest {
    identity_id: String,
    platform: NativePurchasePlatform,
    app_product_id: String,
    package_id: String,
    transaction_id: Option<String>,
    purchase_token: Option<String>,
    receipt_data: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCreditPurchaseVerifyResponse {
    ok: bool,
    account: MobileCreditAccount,
    credits_added: u32,
    duplicate: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSpendCreditsRequest {
    identity_id: String,
    amount: u32,
    reason: String,
    related_id: Option<String>,
    metadata: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSpendCreditsResponse {
    account: MobileCreditAccount,
    credits_spent: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileMintAuthorizationRequest {
    wallet: String,
    payer: Option<String>,
    collection: Option<String>,
    loom_index: u32,
    invite_code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileMintAuthorizationResponse {
    allowed: bool,
    authorization_id: String,
    collection: String,
    expires_at: String,
    loom_index: u32,
    mode: String,
    owner: String,
    payer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    signature: String,
    sponsor: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    sponsor_payer: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareMobileLoomMintRequest {
    authorization_id: String,
    wallet: String,
    payer: Option<String>,
    collection: Option<String>,
    loom_index: u32,
    metadata_uri: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareMobileLoomMintResponse {
    asset: String,
    authorization_id: String,
    blockhash: String,
    collection: String,
    collection_authority: String,
    last_valid_block_height: u64,
    loom_index: u32,
    mode: String,
    name: String,
    owner: String,
    payer: String,
    transaction_base64: String,
    uri: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordMobileLoomMintRequest {
    wallet: String,
    loom_asset: String,
    core_collection: String,
    signature: String,
    loom_index: Option<u32>,
    mint_mode: Option<String>,
    metadata_uri: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordMobileLoomMintResponse {
    recorded: bool,
    loom: MobileLoomMint,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileLoomMint {
    id: String,
    network: String,
    wallet: String,
    loom_asset: String,
    core_collection: String,
    signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    loom_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mint_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_uri: Option<String>,
    status: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileLoomLookupQuery {
    wallet: String,
}

#[derive(Debug, Serialize)]
pub struct MobileLoomLookupResponse {
    looms: Vec<MobileLoomMint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MobileThreadMode {
    Fragment,
    Complete,
    Reflection,
}

impl MobileThreadMode {
    fn as_str(self) -> &'static str {
        match self {
            MobileThreadMode::Fragment => "fragment",
            MobileThreadMode::Complete => "complete",
            MobileThreadMode::Reflection => "reflection",
        }
    }

    fn seed_instruction(self) -> &'static str {
        match self {
            MobileThreadMode::Fragment => {
                "this stopped before it became complete. that does not make it empty. speak to the unfinished thread."
            }
            MobileThreadMode::Complete => {
                "this is a complete anky. sit beside it. reflect the living thread, not as analysis, but as witness."
            }
            MobileThreadMode::Reflection => {
                "a mirror has already been given. continue from there. help the user stay with what still has heat."
            }
        }
    }
}

#[derive(Debug, Clone)]
struct MobileThreadInput {
    session_hash: String,
    mode: MobileThreadMode,
    raw_anky: String,
    reconstructed_text: String,
    existing_reflection: Option<String>,
    messages: Vec<MobileThreadInputMessage>,
    user_message: String,
}

#[derive(Debug, Clone)]
struct MobileThreadInputMessage {
    role: MobileThreadRole,
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MobileThreadRole {
    User,
    Anky,
}

impl MobileThreadRole {
    fn provider_role(self) -> &'static str {
        match self {
            MobileThreadRole::User => "user",
            MobileThreadRole::Anky => "assistant",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileThreadResponse {
    message: MobileThreadResponseMessage,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileThreadResponseMessage {
    role: String,
    content: String,
    created_at: String,
}

#[derive(Debug)]
pub enum MobileThreadError {
    App(AppError),
    ThreadUnavailable,
}

impl From<AppError> for MobileThreadError {
    fn from(error: AppError) -> Self {
        Self::App(error)
    }
}

impl IntoResponse for MobileThreadError {
    fn into_response(self) -> Response {
        match self {
            MobileThreadError::App(error) => error.into_response(),
            MobileThreadError::ThreadUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                json!({
                    "error": "thread_unavailable",
                    "message": "anky cannot continue the thread right now."
                })
                .to_string(),
            )
                .into_response(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileReflectionRequest {
    identity_id: String,
    session_hash: String,
    anky: String,
    processing_type: Option<ProcessingType>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileReflectionResponse {
    artifacts: Vec<Value>,
    credits_remaining: u32,
    credits_spent: u32,
    job: MobileReflectionJob,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileReflectionJobResponse {
    job: MobileReflectionJob,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileReflectionJob {
    id: String,
    identity_id: String,
    session_hash: String,
    processing_type: String,
    status: String,
    credits_spent: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    request: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordMobileSealRequest {
    wallet: String,
    loom_asset: String,
    core_collection: String,
    session_hash: String,
    signature: String,
    utc_day: Option<i64>,
    slot: Option<u64>,
    block_time: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordMobileSealResponse {
    recorded: bool,
    seal: LoomSeal,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordMobileVerifiedSealRequest {
    wallet: String,
    session_hash: String,
    proof_hash: String,
    verifier: String,
    protocol_version: u16,
    signature: String,
    utc_day: Option<i64>,
    slot: Option<u64>,
    block_time: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileVerifiedSeal {
    tx_signature: String,
    writer: String,
    session_hash: String,
    proof_hash: String,
    verifier: String,
    protocol_version: u16,
    network: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    utc_day: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block_time: Option<i64>,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordMobileVerifiedSealResponse {
    recorded: bool,
    verified_seal: MobileVerifiedSeal,
    seal: LoomSeal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeliusWebhookEventReceipt {
    id: String,
    network: String,
    source: String,
    payload_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    event_count: u32,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordHeliusAnkySealWebhookResponse {
    recorded: bool,
    event: HeliusWebhookEventReceipt,
}

fn mobile_solana_config() -> MobileSolanaConfigResponse {
    let cluster = solana_cluster();

    MobileSolanaConfigResponse {
        cluster: cluster.clone(),
        network: cluster,
        rpc_url: public_solana_rpc_url(),
        core_program_id: core_program_id(),
        core_collection: core_collection(),
        seal_program_id: seal_program_id(),
        proof_verifier_authority: proof_verifier_authority(),
        collection_uri: collection_uri(),
        loom_metadata_base_url: loom_metadata_base_url(),
        seal_verification: seal_verification_label(),
    }
}

const MAX_MOBILE_THREAD_MESSAGES: usize = 32;
const MAX_MOBILE_THREAD_MESSAGE_CHARS: usize = 4_000;
const MAX_MOBILE_THREAD_USER_MESSAGE_CHARS: usize = 3_000;
const MAX_MOBILE_THREAD_RAW_ANKY_CHARS: usize = 250_000;
const MAX_MOBILE_THREAD_RECONSTRUCTED_CHARS: usize = 60_000;
const MAX_MOBILE_THREAD_REFLECTION_CHARS: usize = 30_000;
const MAX_MOBILE_THREAD_REPLY_CHARS: usize = 2_400;

fn validate_mobile_thread_payload(payload: Value) -> Result<MobileThreadInput, AppError> {
    let object = payload
        .as_object()
        .ok_or_else(|| AppError::BadRequest("request body must be a JSON object".into()))?;
    let session_hash = normalize_hash(required_string_field(object, "sessionHash")?)?;
    let mode = validate_mobile_thread_mode(required_string_field(object, "mode")?)?;
    let raw_anky =
        validate_required_text_field(object, "rawAnky", MAX_MOBILE_THREAD_RAW_ANKY_CHARS)?;
    let reconstructed_text = validate_required_text_field(
        object,
        "reconstructedText",
        MAX_MOBILE_THREAD_RECONSTRUCTED_CHARS,
    )?;
    let existing_reflection = validate_optional_text_field(
        object,
        "existingReflection",
        MAX_MOBILE_THREAD_REFLECTION_CHARS,
    )?;
    let user_message =
        validate_required_text_field(object, "userMessage", MAX_MOBILE_THREAD_USER_MESSAGE_CHARS)?;
    let messages_value = object
        .get("messages")
        .ok_or_else(|| AppError::BadRequest("messages is required".into()))?;
    let messages_array = messages_value
        .as_array()
        .ok_or_else(|| AppError::BadRequest("messages must be an array".into()))?;

    if messages_array.len() > MAX_MOBILE_THREAD_MESSAGES {
        return Err(AppError::BadRequest("messages has too many items".into()));
    }

    let mut messages = Vec::with_capacity(messages_array.len());
    for message in messages_array {
        let message_object = message
            .as_object()
            .ok_or_else(|| AppError::BadRequest("messages must contain objects".into()))?;
        let role = validate_mobile_thread_role(required_string_field(message_object, "role")?)?;
        let content = validate_required_text_field_from_object(
            message_object,
            "content",
            MAX_MOBILE_THREAD_MESSAGE_CHARS,
        )?;
        let _created_at =
            validate_required_text_field_from_object(message_object, "createdAt", 128)?;

        messages.push(MobileThreadInputMessage { role, content });
    }

    Ok(MobileThreadInput {
        session_hash,
        mode,
        raw_anky,
        reconstructed_text,
        existing_reflection,
        messages,
        user_message,
    })
}

fn validate_mobile_thread_mode(value: &str) -> Result<MobileThreadMode, AppError> {
    match value.trim() {
        "fragment" => Ok(MobileThreadMode::Fragment),
        "complete" => Ok(MobileThreadMode::Complete),
        "reflection" => Ok(MobileThreadMode::Reflection),
        _ => Err(AppError::BadRequest("mode is invalid".into())),
    }
}

fn validate_mobile_thread_role(value: &str) -> Result<MobileThreadRole, AppError> {
    match value.trim() {
        "user" => Ok(MobileThreadRole::User),
        "anky" => Ok(MobileThreadRole::Anky),
        _ => Err(AppError::BadRequest("message role is invalid".into())),
    }
}

fn required_string_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
) -> Result<&'a str, AppError> {
    object
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::BadRequest(format!("{name} is required")))
}

fn validate_required_text_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
    max_chars: usize,
) -> Result<String, AppError> {
    validate_required_text_field_from_object(object, name, max_chars)
}

fn validate_required_text_field_from_object(
    object: &serde_json::Map<String, Value>,
    name: &str,
    max_chars: usize,
) -> Result<String, AppError> {
    let value = required_string_field(object, name)?.trim();

    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{name} is required")));
    }
    if value.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!("{name} is too long")));
    }

    Ok(value.to_string())
}

fn validate_optional_text_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
    max_chars: usize,
) -> Result<Option<String>, AppError> {
    let Some(value) = object.get(name) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(text) = value.as_str() else {
        return Err(AppError::BadRequest(format!("{name} must be a string")));
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!("{name} is too long")));
    }

    Ok(Some(trimmed.to_string()))
}

fn build_mobile_thread_system_prompt(req: &MobileThreadInput) -> String {
    let existing_reflection = req
        .existing_reflection
        .as_deref()
        .unwrap_or("no prior reflection was included.");
    let raw_anky_note = format!(
        ".anky artifact received: {} bytes. use the reconstructed writing below as the readable text; do not quote or expose the raw artifact format.",
        req.raw_anky.len()
    );

    format!(
        r#"you are anky, continuing an artifact-attached private writing conversation.

this is not generic chat. this conversation is anchored to one session hash:
{session_hash}

mode: {mode}
seed instruction: {seed_instruction}

voice:
- gentle, precise, poetic but grounded
- curious, non-clinical, not a therapist
- witness the living thread instead of analyzing mechanically
- no productivity advice unless the user clearly asks for it
- never say "as an ai language model"
- never open with "how can i help you today?"
- lowercase is preferred

response shape:
- return one anky message only
- 80 to 180 words is ideal; shorter is better
- one strong image or one precise question is better than many ideas
- usually end with one question

safety:
if the user appears to be in immediate danger or may harm themself, be direct and grounded. say you cannot be emergency support from here, ask them to contact local emergency services or a trusted person nearby now, and keep it brief.

privacy:
the writing is private processing input. do not claim it is public. do not mention storing it. do not talk about chains, wallets, or proofs unless the user asks.

{raw_anky_note}

reconstructed writing:
{reconstructed_text}

existing reflection, if any:
{existing_reflection}"#,
        session_hash = req.session_hash,
        mode = req.mode.as_str(),
        seed_instruction = req.mode.seed_instruction(),
        raw_anky_note = raw_anky_note,
        reconstructed_text = req.reconstructed_text,
        existing_reflection = existing_reflection,
    )
}

fn build_mobile_thread_provider_messages(req: &MobileThreadInput) -> Vec<(String, String)> {
    let mut messages: Vec<(String, String)> = req
        .messages
        .iter()
        .map(|message| {
            (
                message.role.provider_role().to_string(),
                message.content.clone(),
            )
        })
        .collect();

    messages.push(("user".to_string(), req.user_message.clone()));
    messages
}

fn normalize_mobile_thread_reply(text: &str) -> Result<String, &'static str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("empty_reply");
    }

    let mut content: String = trimmed
        .chars()
        .take(MAX_MOBILE_THREAD_REPLY_CHARS)
        .collect();
    let lower = content.to_ascii_lowercase();
    let banned_openings = [
        "as an ai language model",
        "how can i help you today",
        "here are some productivity tips",
    ];

    if banned_openings
        .iter()
        .any(|opening| lower.contains(opening))
    {
        return Err("chatbot_like_reply");
    }

    if trimmed.chars().count() > MAX_MOBILE_THREAD_REPLY_CHARS {
        content = format!("{}...", content.trim_end());
    }

    Ok(content)
}

fn mobile_thread_response(content: String) -> MobileThreadResponse {
    MobileThreadResponse {
        message: MobileThreadResponseMessage {
            role: "anky".to_string(),
            content,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    }
}

fn mobile_thread_needs_immediate_safety_response(req: &MobileThreadInput) -> bool {
    let text = format!(
        "{}\n{}\n{}",
        req.user_message,
        req.reconstructed_text,
        req.existing_reflection.as_deref().unwrap_or("")
    )
    .to_ascii_lowercase();
    let indicators = [
        "kill myself",
        "end my life",
        "want to die",
        "i want to die",
        "suicide",
        "hurt myself",
        "harm myself",
        "not safe with myself",
    ];

    indicators.iter().any(|indicator| text.contains(indicator))
}

fn mobile_thread_safety_response() -> String {
    "i am here with you, and this sounds immediate. i cannot be emergency support from here. please contact your local emergency number now, or reach someone physically near you and say the plain thing: i need help right now. if there is anything near you that you could use to hurt yourself, move away from it before continuing this thread. can you pause here and reach a real person now?".to_string()
}

struct MobileMintAuthorizationRecord {
    authorization_id: String,
    mode: String,
}

struct PreparedCoreLoomMintTransaction {
    asset: String,
    blockhash: String,
    collection_authority: String,
    last_valid_block_height: u64,
    transaction_base64: String,
}

#[derive(Debug, Deserialize)]
struct LatestBlockhashRpcResponse {
    result: Option<LatestBlockhashRpcResult>,
    error: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct LatestBlockhashRpcResult {
    value: LatestBlockhashRpcValue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestBlockhashRpcValue {
    blockhash: String,
    last_valid_block_height: u64,
}

#[derive(Debug, Deserialize)]
struct SolanaAccountRpcResponse {
    result: Option<SolanaAccountRpcResult>,
    error: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct SolanaAccountRpcResult {
    value: Option<SolanaAccountValue>,
}

#[derive(Debug, Deserialize)]
struct SolanaAccountValue {
    data: Value,
}

async fn lookup_mobile_mint_authorization(
    pool: &sqlx::PgPool,
    authorization_id: &str,
    wallet: &str,
    payer: &str,
    collection: &str,
    loom_index: u32,
) -> Result<MobileMintAuthorizationRecord, AppError> {
    let authorization_id = validate_short_text("authorizationId", authorization_id, 128)?;
    let row = sqlx::query(
        "SELECT id, mode, allowed, expires_at
         FROM mobile_mint_authorizations
         WHERE id = $1
           AND network = $2
           AND wallet = $3
           AND payer = $4
           AND core_collection = $5
           AND loom_index = $6",
    )
    .bind(&authorization_id)
    .bind(solana_cluster())
    .bind(wallet)
    .bind(payer)
    .bind(collection)
    .bind(loom_index as i32)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("mint authorization was not found".into()))?;

    let allowed: bool = row.try_get("allowed")?;
    if !allowed {
        return Err(AppError::Forbidden(
            "mint authorization is not allowed".into(),
        ));
    }

    let expires_at: chrono::DateTime<chrono::Utc> = row.try_get("expires_at")?;
    if chrono::Utc::now() > expires_at {
        return Err(AppError::BadRequest("mint authorization expired".into()));
    }

    Ok(MobileMintAuthorizationRecord {
        authorization_id: row.try_get("id")?,
        mode: row.try_get("mode")?,
    })
}

async fn build_core_loom_mint_transaction(
    wallet: &str,
    payer: &str,
    collection: &str,
    name: &str,
    uri: &str,
) -> Result<PreparedCoreLoomMintTransaction, AppError> {
    if solana_cluster() == "mainnet-beta" && !env_flag("ANKY_ENABLE_MAINNET_LOOM_MINTS") {
        return Err(AppError::Unavailable(
            "mainnet Loom mint preparation requires ANKY_ENABLE_MAINNET_LOOM_MINTS=true".into(),
        ));
    }

    let owner_pubkey = solana_pubkey("wallet", wallet)?;
    let payer_pubkey = solana_pubkey("payer", payer)?;
    let collection_pubkey = solana_pubkey("collection", collection)?;
    let collection_authority = load_core_collection_authority_keypair()?;
    let asset = SolanaKeypair::new();
    let latest_blockhash = fetch_latest_blockhash().await?;
    let recent_blockhash = SolanaHash::from_str(&latest_blockhash.blockhash)
        .map_err(|_| AppError::Internal("RPC returned an invalid blockhash".into()))?;

    let instruction = CreateV2Builder::new()
        .asset(asset.pubkey())
        .collection(Some(collection_pubkey))
        .authority(Some(collection_authority.pubkey()))
        .payer(payer_pubkey)
        .owner(Some(owner_pubkey))
        .name(name.to_string())
        .uri(uri.to_string())
        .instruction();
    let mut transaction = SolanaTransaction::new_with_payer(&[instruction], Some(&payer_pubkey));
    transaction.partial_sign(&[&asset, &collection_authority], recent_blockhash);
    let serialized = bincode::serialize(&transaction).map_err(|error| {
        AppError::Internal(format!(
            "could not serialize Loom mint transaction: {error}"
        ))
    })?;

    Ok(PreparedCoreLoomMintTransaction {
        asset: asset.pubkey().to_string(),
        blockhash: latest_blockhash.blockhash,
        collection_authority: collection_authority.pubkey().to_string(),
        last_valid_block_height: latest_blockhash.last_valid_block_height,
        transaction_base64: BASE64_STANDARD.encode(serialized),
    })
}

async fn fetch_latest_blockhash() -> Result<LatestBlockhashRpcValue, AppError> {
    let response = reqwest::Client::new()
        .post(solana_rpc_url())
        .json(&json!({
            "jsonrpc": "2.0",
            "id": "anky-loom-mint-blockhash",
            "method": "getLatestBlockhash",
            "params": [{ "commitment": "confirmed" }]
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<LatestBlockhashRpcResponse>()
        .await?;

    if let Some(error) = response.error {
        return Err(AppError::Unavailable(format!(
            "Solana RPC getLatestBlockhash failed: {error}"
        )));
    }

    response
        .result
        .map(|result| result.value)
        .ok_or_else(|| AppError::Unavailable("Solana RPC returned no blockhash".into()))
}

fn load_core_collection_authority_keypair() -> Result<SolanaKeypair, AppError> {
    if let Some(value) = env_nonempty("ANKY_CORE_COLLECTION_AUTHORITY_KEYPAIR") {
        return parse_solana_keypair(&value);
    }

    let keypair_path = env_nonempty("ANKY_CORE_COLLECTION_AUTHORITY_KEYPAIR_PATH")
        .or_else(|| env_nonempty("KEYPAIR_PATH"));
    let Some(keypair_path) = keypair_path else {
        return Err(AppError::Unavailable(
            "ANKY_CORE_COLLECTION_AUTHORITY_KEYPAIR or ANKY_CORE_COLLECTION_AUTHORITY_KEYPAIR_PATH is required to prepare official Loom mint transactions".into(),
        ));
    };

    let path = expand_home(&keypair_path);
    let value = std::fs::read_to_string(&path).map_err(|error| {
        AppError::Unavailable(format!(
            "could not read Core collection authority keypair at {path}: {error}"
        ))
    })?;

    parse_solana_keypair(&value)
}

fn parse_solana_keypair(value: &str) -> Result<SolanaKeypair, AppError> {
    let trimmed = value.trim();
    let bytes = if trimmed.starts_with('[') {
        let values = serde_json::from_str::<Vec<u8>>(trimmed)
            .map_err(|_| AppError::BadRequest("keypair JSON must be a byte array".into()))?;
        values
    } else {
        bs58::decode(trimmed).into_vec().map_err(|_| {
            AppError::BadRequest("keypair must be a base58 secret key or JSON byte array".into())
        })?
    };

    if bytes.len() != 64 {
        return Err(AppError::BadRequest(
            "Core collection authority keypair must contain 64 bytes".into(),
        ));
    }

    SolanaKeypair::try_from(bytes.as_slice())
        .map_err(|_| AppError::BadRequest("Core collection authority keypair is invalid".into()))
}

fn solana_pubkey(name: &str, value: &str) -> Result<SolanaPubkey, AppError> {
    let validated = validate_public_key(name, value)?;
    SolanaPubkey::from_str(&validated)
        .map_err(|_| AppError::BadRequest(format!("{name} is not a valid Solana public key")))
}

fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = env_nonempty("HOME") {
            return format!("{home}/{rest}");
        }
    }

    path.to_string()
}

struct NativeCreditPackage {
    package_id: &'static str,
    ios_product_id: &'static str,
    android_product_id: &'static str,
    credits_granted: u32,
}

fn native_credit_package(package_id: &str) -> Option<NativeCreditPackage> {
    match package_id.trim() {
        "credits_22" => Some(NativeCreditPackage {
            android_product_id: "credits_22",
            credits_granted: 22,
            ios_product_id: "inc.anky.credits.22",
            package_id: "credits_22",
        }),
        "credits_88_bonus_11" => Some(NativeCreditPackage {
            android_product_id: "credits_88_bonus_11",
            credits_granted: 99,
            ios_product_id: "inc.anky.credits.88_bonus_11",
            package_id: "credits_88_bonus_11",
        }),
        "credits_333_bonus_88" => Some(NativeCreditPackage {
            android_product_id: "credits_333_bonus_88",
            credits_granted: 421,
            ios_product_id: "inc.anky.credits.333_bonus_88",
            package_id: "credits_333_bonus_88",
        }),
        _ => None,
    }
}

fn validate_native_credit_purchase_request(
    req: &NativeCreditPurchaseVerifyRequest,
    package: &NativeCreditPackage,
) -> Result<(), AppError> {
    let expected_product_id = match req.platform {
        NativePurchasePlatform::Ios => package.ios_product_id,
        NativePurchasePlatform::Android => package.android_product_id,
    };

    if req.app_product_id.trim() != expected_product_id {
        return Err(AppError::BadRequest(
            "native product id does not match packageId".into(),
        ));
    }

    match req.platform {
        NativePurchasePlatform::Ios => {
            if first_nonempty([
                req.transaction_id.as_deref(),
                req.purchase_token.as_deref(),
                req.receipt_data.as_deref(),
            ])
            .is_none()
            {
                return Err(AppError::BadRequest(
                    "iOS purchase evidence is required".into(),
                ));
            }
        }
        NativePurchasePlatform::Android => {
            if req
                .purchase_token
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                return Err(AppError::BadRequest(
                    "Android purchaseToken is required".into(),
                ));
            }
        }
    }

    Ok(())
}

async fn verify_native_store_purchase(
    req: &NativeCreditPurchaseVerifyRequest,
) -> Result<(), AppError> {
    if env_flag("ANKY_NATIVE_IAP_DEV_BYPASS") && dev_plaintext_processing_allowed() {
        tracing::warn!(
            package_id = %req.package_id,
            platform = %req.platform.as_str(),
            "native credit purchase verification bypassed for development"
        );
        return Ok(());
    }

    match req.platform {
        NativePurchasePlatform::Ios => verify_apple_native_purchase(req).await,
        NativePurchasePlatform::Android => verify_google_native_purchase(req).await,
    }
}

#[derive(Debug, Serialize)]
struct AppleServerApiClaims {
    iss: String,
    iat: i64,
    exp: i64,
    aud: String,
    bid: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppleTransactionInfoResponse {
    signed_transaction_info: String,
}

async fn verify_apple_native_purchase(
    req: &NativeCreditPurchaseVerifyRequest,
) -> Result<(), AppError> {
    let transaction_id = apple_transaction_id(req)?;
    let token = apple_server_api_token()?;
    let client = reqwest::Client::new();
    let base_urls = apple_store_api_base_urls();

    for base_url in base_urls {
        let url = format!(
            "{}/inApps/v1/transactions/{}",
            base_url,
            urlencoding::encode(&transaction_id)
        );
        let response = client.get(url).bearer_auth(&token).send().await?;
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            let info: AppleTransactionInfoResponse = serde_json::from_str(&body)?;
            let transaction = decode_jws_payload(&info.signed_transaction_info)?;
            let product_id = transaction
                .get("productId")
                .and_then(Value::as_str)
                .unwrap_or("");
            let bundle_id = transaction
                .get("bundleId")
                .and_then(Value::as_str)
                .unwrap_or("");
            let returned_transaction_id = transaction
                .get("transactionId")
                .and_then(Value::as_str)
                .unwrap_or("");

            if product_id != req.app_product_id.trim() {
                return Err(AppError::BadRequest(
                    "Apple transaction product does not match packageId".into(),
                ));
            }

            if returned_transaction_id != transaction_id {
                return Err(AppError::BadRequest(
                    "Apple transaction id does not match purchase evidence".into(),
                ));
            }

            if let Some(expected_bundle_id) = env_nonempty("ANKY_IOS_BUNDLE_ID") {
                if bundle_id != expected_bundle_id {
                    return Err(AppError::BadRequest(
                        "Apple transaction bundle id does not match".into(),
                    ));
                }
            }

            if transaction.get("revocationDate").is_some() {
                return Err(AppError::BadRequest(
                    "Apple transaction has been revoked".into(),
                ));
            }

            return Ok(());
        }

        if status.as_u16() != 404 {
            tracing::warn!(%status, body = %body, "Apple native purchase verification failed");
            return Err(AppError::BadRequest(
                "Apple could not verify this purchase".into(),
            ));
        }
    }

    Err(AppError::BadRequest(
        "Apple could not find this purchase".into(),
    ))
}

fn apple_transaction_id(req: &NativeCreditPurchaseVerifyRequest) -> Result<String, AppError> {
    if let Some(transaction_id) = req.transaction_id.as_deref().map(str::trim) {
        if !transaction_id.is_empty() {
            return Ok(transaction_id.to_string());
        }
    }

    for signed_payload in [req.purchase_token.as_deref(), req.receipt_data.as_deref()]
        .into_iter()
        .flatten()
    {
        let payload = decode_jws_payload(signed_payload)?;

        if let Some(transaction_id) = payload.get("transactionId").and_then(Value::as_str) {
            if !transaction_id.trim().is_empty() {
                return Ok(transaction_id.trim().to_string());
            }
        }
    }

    Err(AppError::BadRequest(
        "Apple transactionId is required".into(),
    ))
}

fn apple_server_api_token() -> Result<String, AppError> {
    let issuer_id = env_nonempty("ANKY_APP_STORE_ISSUER_ID")
        .ok_or_else(|| AppError::Unavailable("Apple IAP issuer id is not configured".into()))?;
    let key_id = env_nonempty("ANKY_APP_STORE_KEY_ID")
        .ok_or_else(|| AppError::Unavailable("Apple IAP key id is not configured".into()))?;
    let bundle_id = env_nonempty("ANKY_IOS_BUNDLE_ID")
        .ok_or_else(|| AppError::Unavailable("Apple bundle id is not configured".into()))?;
    let private_key = native_private_key_from_env(
        "ANKY_APP_STORE_PRIVATE_KEY",
        "ANKY_APP_STORE_PRIVATE_KEY_PATH",
    )?;
    let now = chrono::Utc::now().timestamp();
    let claims = AppleServerApiClaims {
        aud: "appstoreconnect-v1".to_string(),
        bid: bundle_id,
        exp: now + 20 * 60,
        iat: now,
        iss: issuer_id,
    };
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key_id);

    jwt_encode(
        &header,
        &claims,
        &EncodingKey::from_ec_pem(private_key.as_bytes()).map_err(|error| {
            AppError::Unavailable(format!("Apple IAP private key is invalid: {error}"))
        })?,
    )
    .map_err(|error| AppError::Unavailable(format!("Apple IAP token signing failed: {error}")))
}

fn apple_store_api_base_urls() -> Vec<&'static str> {
    match env_nonempty("ANKY_APP_STORE_ENVIRONMENT")
        .unwrap_or_else(|| "auto".to_string())
        .as_str()
    {
        "production" => vec!["https://api.storekit.itunes.apple.com"],
        "sandbox" => vec!["https://api.storekit-sandbox.itunes.apple.com"],
        _ => vec![
            "https://api.storekit.itunes.apple.com",
            "https://api.storekit-sandbox.itunes.apple.com",
        ],
    }
}

#[derive(Debug, Deserialize)]
struct GoogleServiceAccount {
    client_email: String,
    private_key: String,
    token_uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct GoogleOAuthClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Deserialize)]
struct GoogleOAuthTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleProductPurchaseResponse {
    purchase_state: Option<i32>,
}

async fn verify_google_native_purchase(
    req: &NativeCreditPurchaseVerifyRequest,
) -> Result<(), AppError> {
    let package_name = env_nonempty("ANKY_GOOGLE_PLAY_PACKAGE_NAME").ok_or_else(|| {
        AppError::Unavailable("Google Play package name is not configured".into())
    })?;
    let purchase_token = req
        .purchase_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::BadRequest("Android purchaseToken is required".into()))?;
    let access_token = google_play_access_token().await?;
    let url = format!(
        "https://androidpublisher.googleapis.com/androidpublisher/v3/applications/{}/purchases/products/{}/tokens/{}",
        urlencoding::encode(&package_name),
        urlencoding::encode(req.app_product_id.trim()),
        urlencoding::encode(purchase_token),
    );
    let response = reqwest::Client::new()
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        tracing::warn!(%status, body = %body, "Google native purchase verification failed");
        return Err(AppError::BadRequest(
            "Google Play could not verify this purchase".into(),
        ));
    }

    let purchase: GoogleProductPurchaseResponse = serde_json::from_str(&body)?;

    if purchase.purchase_state.unwrap_or(1) != 0 {
        return Err(AppError::BadRequest(
            "Google Play purchase is not completed".into(),
        ));
    }

    Ok(())
}

async fn google_play_access_token() -> Result<String, AppError> {
    let account = google_service_account()?;
    let token_uri = account
        .token_uri
        .unwrap_or_else(|| "https://oauth2.googleapis.com/token".to_string());
    let now = chrono::Utc::now().timestamp();
    let claims = GoogleOAuthClaims {
        aud: token_uri.clone(),
        exp: now + 55 * 60,
        iat: now,
        iss: account.client_email,
        scope: "https://www.googleapis.com/auth/androidpublisher".to_string(),
    };
    let header = Header::new(Algorithm::RS256);
    let assertion = jwt_encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(account.private_key.as_bytes()).map_err(|error| {
            AppError::Unavailable(format!("Google Play private key is invalid: {error}"))
        })?,
    )
    .map_err(|error| AppError::Unavailable(format!("Google Play token signing failed: {error}")))?;
    let response = reqwest::Client::new()
        .post(token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", assertion.as_str()),
        ])
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        tracing::warn!(%status, body = %body, "Google Play OAuth failed");
        return Err(AppError::Unavailable(
            "Google Play verification auth failed".into(),
        ));
    }

    let token: GoogleOAuthTokenResponse = serde_json::from_str(&body)?;

    Ok(token.access_token)
}

fn google_service_account() -> Result<GoogleServiceAccount, AppError> {
    let raw = if let Some(json) = env_nonempty("ANKY_GOOGLE_PLAY_SERVICE_ACCOUNT_JSON") {
        if json.trim_start().starts_with('{') {
            json
        } else {
            std::fs::read_to_string(json)?
        }
    } else if let Some(path) = env_nonempty("ANKY_GOOGLE_PLAY_SERVICE_ACCOUNT_PATH")
        .or_else(|| env_nonempty("GOOGLE_APPLICATION_CREDENTIALS"))
    {
        std::fs::read_to_string(path)?
    } else {
        return Err(AppError::Unavailable(
            "Google Play service account is not configured".into(),
        ));
    };

    serde_json::from_str(&raw).map_err(AppError::from)
}

fn native_private_key_from_env(value_name: &str, path_name: &str) -> Result<String, AppError> {
    if let Some(value) = env_nonempty(value_name) {
        if value.trim_start().starts_with("-----BEGIN") {
            return Ok(value.replace("\\n", "\n"));
        }

        return Ok(std::fs::read_to_string(value)?);
    }

    if let Some(path) = env_nonempty(path_name) {
        return Ok(std::fs::read_to_string(path)?);
    }

    Err(AppError::Unavailable(format!(
        "{value_name} or {path_name} is required"
    )))
}

fn decode_jws_payload(token: &str) -> Result<Value, AppError> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or_else(|| AppError::BadRequest("signed purchase payload is malformed".into()))?;
    let bytes = BASE64_URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AppError::BadRequest("signed purchase payload is malformed".into()))?;

    serde_json::from_slice(&bytes).map_err(AppError::from)
}

async fn grant_native_mobile_credits(
    pool: &sqlx::PgPool,
    identity_id: &str,
    req: &NativeCreditPurchaseVerifyRequest,
    package: &NativeCreditPackage,
) -> Result<(MobileCreditAccount, u32, bool), AppError> {
    ensure_mobile_credit_account(pool, identity_id).await?;

    let platform = req.platform.as_str();
    let purchase_key = native_purchase_key(req)?;
    let raw_receipt_json = serde_json::to_string(req)?;
    let mut tx = pool.begin().await?;

    let existing = sqlx::query(
        "SELECT id
         FROM mobile_credit_purchases
         WHERE platform = $1
           AND purchase_key = $2",
    )
    .bind(platform)
    .bind(&purchase_key)
    .fetch_optional(&mut *tx)
    .await?;

    if existing.is_some() {
        let account = select_mobile_credit_account(&mut tx, identity_id).await?;
        tx.commit().await?;

        return Ok((account, 0, true));
    }

    let purchase_id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO mobile_credit_purchases
         (id, identity_id, platform, app_product_id, package_id, purchase_key, credits_granted, verification_status, raw_receipt_json)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'verified', $8)",
    )
    .bind(&purchase_id)
    .bind(identity_id)
    .bind(platform)
    .bind(req.app_product_id.trim())
    .bind(package.package_id)
    .bind(&purchase_key)
    .bind(package.credits_granted as i32)
    .bind(raw_receipt_json)
    .execute(&mut *tx)
    .await?;

    let row = sqlx::query(
        "UPDATE mobile_credit_accounts
         SET credits_remaining = credits_remaining + $2,
             updated_at = NOW()
         WHERE identity_id = $1
         RETURNING identity_id, credits_remaining, created_at, updated_at",
    )
    .bind(identity_id)
    .bind(package.credits_granted as i32)
    .fetch_one(&mut *tx)
    .await?;
    let account = mobile_credit_account_from_row(&row)?;

    sqlx::query(
        "INSERT INTO mobile_credit_events (id, identity_id, delta, reason, related_id, metadata_json)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(identity_id)
    .bind(package.credits_granted as i32)
    .bind("native_purchase")
    .bind(&purchase_id)
    .bind(
        json!({
            "appProductId": req.app_product_id,
            "packageId": package.package_id,
            "platform": platform,
        })
        .to_string(),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((account, package.credits_granted, false))
}

async fn select_mobile_credit_account(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    identity_id: &str,
) -> Result<MobileCreditAccount, AppError> {
    let row = sqlx::query(
        "SELECT identity_id, credits_remaining, created_at, updated_at
         FROM mobile_credit_accounts
         WHERE identity_id = $1",
    )
    .bind(identity_id)
    .fetch_one(&mut **tx)
    .await?;

    mobile_credit_account_from_row(&row)
}

fn native_purchase_key(req: &NativeCreditPurchaseVerifyRequest) -> Result<String, AppError> {
    first_nonempty([
        req.transaction_id.as_deref(),
        req.purchase_token.as_deref(),
        req.receipt_data.as_deref(),
    ])
    .map(ToString::to_string)
    .ok_or_else(|| AppError::BadRequest("purchase evidence is required".into()))
}

fn first_nonempty<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<&'a str> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
}

async fn ensure_mobile_credit_account(
    pool: &sqlx::PgPool,
    identity_id: &str,
) -> Result<MobileCreditAccount, AppError> {
    sqlx::query(
        "INSERT INTO mobile_credit_accounts (identity_id, credits_remaining)
         VALUES ($1, $2)
         ON CONFLICT (identity_id) DO NOTHING",
    )
    .bind(identity_id)
    .bind(initial_mobile_credits() as i32)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT identity_id, credits_remaining, created_at, updated_at
         FROM mobile_credit_accounts
         WHERE identity_id = $1",
    )
    .bind(identity_id)
    .fetch_one(pool)
    .await?;

    mobile_credit_account_from_row(&row)
}

async fn debit_mobile_credits(
    pool: &sqlx::PgPool,
    identity_id: &str,
    amount: u32,
    reason: &str,
    related_id: Option<&str>,
    metadata: Value,
) -> Result<MobileCreditAccount, AppError> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "UPDATE mobile_credit_accounts
         SET credits_remaining = credits_remaining - $2,
             updated_at = NOW()
         WHERE identity_id = $1
           AND credits_remaining >= $2
         RETURNING identity_id, credits_remaining, created_at, updated_at",
    )
    .bind(identity_id)
    .bind(amount as i32)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::PaymentRequired("not enough credits".into()))?;

    let account = mobile_credit_account_from_row(&row)?;

    sqlx::query(
        "INSERT INTO mobile_credit_events (id, identity_id, delta, reason, related_id, metadata_json)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(identity_id)
    .bind(-(amount as i32))
    .bind(reason)
    .bind(related_id)
    .bind(metadata.to_string())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(account)
}

async fn query_seal_receipts(
    pool: &sqlx::PgPool,
    query: &SealLookupQuery,
) -> Result<Vec<LoomSeal>, AppError> {
    let network = solana_cluster();
    let rows = if let Some(wallet) = query.wallet.as_deref() {
        let wallet = validate_public_key("wallet", wallet)?;
        sqlx::query(
            "SELECT msr.id, msr.network, msr.wallet, msr.loom_asset, msr.core_collection, msr.session_hash, msr.signature, msr.utc_day, msr.slot, msr.block_time, msr.status, msr.created_at,
                    verified.proof_hash, verified.signature AS proof_signature, verified.verifier AS proof_verifier,
                    verified.protocol_version AS proof_protocol_version, verified.utc_day AS proof_utc_day, verified.slot AS proof_slot,
                    verified.block_time AS proof_block_time, verified.status AS proof_status,
                    verified.created_at AS proof_created_at
             FROM mobile_seal_receipts msr
             LEFT JOIN LATERAL (
                 SELECT proof_hash, signature, verifier, protocol_version, utc_day, slot, block_time, status, created_at
                 FROM mobile_verified_seal_receipts
                 WHERE network = msr.network AND wallet = msr.wallet AND session_hash = msr.session_hash
                 ORDER BY created_at DESC
                 LIMIT 1
             ) verified ON TRUE
             WHERE msr.network = $1 AND msr.wallet = $2
             ORDER BY msr.created_at DESC
             LIMIT 100",
        )
        .bind(&network)
        .bind(wallet)
        .fetch_all(pool)
        .await?
    } else if let Some(loom_id) = query.loom_id.as_deref() {
        let loom_id = validate_public_key("loomId", loom_id)?;
        sqlx::query(
            "SELECT msr.id, msr.network, msr.wallet, msr.loom_asset, msr.core_collection, msr.session_hash, msr.signature, msr.utc_day, msr.slot, msr.block_time, msr.status, msr.created_at,
                    verified.proof_hash, verified.signature AS proof_signature, verified.verifier AS proof_verifier,
                    verified.protocol_version AS proof_protocol_version, verified.utc_day AS proof_utc_day, verified.slot AS proof_slot,
                    verified.block_time AS proof_block_time, verified.status AS proof_status,
                    verified.created_at AS proof_created_at
             FROM mobile_seal_receipts msr
             LEFT JOIN LATERAL (
                 SELECT proof_hash, signature, verifier, protocol_version, utc_day, slot, block_time, status, created_at
                 FROM mobile_verified_seal_receipts
                 WHERE network = msr.network AND wallet = msr.wallet AND session_hash = msr.session_hash
                 ORDER BY created_at DESC
                 LIMIT 1
             ) verified ON TRUE
             WHERE msr.network = $1 AND msr.loom_asset = $2
             ORDER BY msr.created_at DESC
             LIMIT 100",
        )
        .bind(&network)
        .bind(loom_id)
        .fetch_all(pool)
        .await?
    } else if let Some(session_hash) = query.session_hash.as_deref() {
        let session_hash = normalize_hash(session_hash)?;
        sqlx::query(
            "SELECT msr.id, msr.network, msr.wallet, msr.loom_asset, msr.core_collection, msr.session_hash, msr.signature, msr.utc_day, msr.slot, msr.block_time, msr.status, msr.created_at,
                    verified.proof_hash, verified.signature AS proof_signature, verified.verifier AS proof_verifier,
                    verified.protocol_version AS proof_protocol_version, verified.utc_day AS proof_utc_day, verified.slot AS proof_slot,
                    verified.block_time AS proof_block_time, verified.status AS proof_status,
                    verified.created_at AS proof_created_at
             FROM mobile_seal_receipts msr
             LEFT JOIN LATERAL (
                 SELECT proof_hash, signature, verifier, protocol_version, utc_day, slot, block_time, status, created_at
                 FROM mobile_verified_seal_receipts
                 WHERE network = msr.network AND wallet = msr.wallet AND session_hash = msr.session_hash
                 ORDER BY created_at DESC
                 LIMIT 1
             ) verified ON TRUE
             WHERE msr.network = $1 AND msr.session_hash = $2
             ORDER BY msr.created_at DESC
             LIMIT 100",
        )
        .bind(&network)
        .bind(session_hash)
        .fetch_all(pool)
        .await?
    } else {
        Vec::new()
    };

    rows.iter()
        .map(loom_seal_from_row)
        .collect::<Result<Vec<_>, _>>()
}

fn validate_seal_lookup_query(query: &SealLookupQuery) -> Result<(), AppError> {
    let filters = [
        query.wallet.as_deref(),
        query.loom_id.as_deref(),
        query.session_hash.as_deref(),
    ];
    let filter_count = filters.iter().filter(|value| value.is_some()).count();

    if filter_count != 1 {
        return Err(AppError::BadRequest(
            "provide exactly one of wallet, loomId, or sessionHash".into(),
        ));
    }

    if let Some(session_hash) = query.session_hash.as_deref() {
        validate_hash(session_hash)?;
    }

    Ok(())
}

fn build_mobile_seal_score(
    wallet: String,
    network: String,
    proof_verifier_authority: String,
    sealed_days: Vec<i64>,
    verified_days: Vec<i64>,
) -> MobileSealScoreResponse {
    let sealed_days = sorted_unique_days(sealed_days);
    let mut verified_days = sorted_unique_days(verified_days);
    verified_days.retain(|day| sealed_days.binary_search(day).is_ok());

    let unique_seal_days = sealed_days.len() as u32;
    let verified_seal_days = verified_days.len() as u32;
    let streak_bonus = compute_seal_streak_bonus(&sealed_days);
    let score = unique_seal_days + verified_seal_days + streak_bonus;

    MobileSealScoreResponse {
        wallet,
        network,
        proof_verifier_authority,
        unique_seal_days,
        verified_seal_days,
        streak_bonus,
        score,
        sealed_days,
        verified_days,
        finalized_only: true,
        formula:
            "score = unique_seal_days + verified_days + 2 * floor(each_consecutive_day_run / 7)",
    }
}

fn sorted_unique_days(mut days: Vec<i64>) -> Vec<i64> {
    days.retain(|day| *day >= 0);
    days.sort_unstable();
    days.dedup();
    days
}

fn compute_seal_streak_bonus(sorted_days: &[i64]) -> u32 {
    let mut bonus = 0u32;
    let mut run_len = 0u32;
    let mut previous_day: Option<i64> = None;

    for day in sorted_days {
        if previous_day.is_some_and(|previous| *day == previous + 1) {
            run_len += 1;
        } else {
            bonus += 2 * (run_len / 7);
            run_len = 1;
        }
        previous_day = Some(*day);
    }

    bonus + 2 * (run_len / 7)
}

fn mobile_credit_account_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<MobileCreditAccount, AppError> {
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    let credits_remaining: i32 = row.try_get("credits_remaining")?;

    Ok(MobileCreditAccount {
        identity_id: row.try_get("identity_id")?,
        credits_remaining: credits_remaining.max(0) as u32,
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
    })
}

fn loom_seal_from_row(row: &sqlx::postgres::PgRow) -> Result<LoomSeal, AppError> {
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let slot: Option<i64> = row.try_get("slot")?;
    let proof_created_at = optional_datetime_column(row, "proof_created_at");
    let proof_slot = optional_i64_column(row, "proof_slot");
    let proof_protocol_version = optional_i32_column(row, "proof_protocol_version")
        .and_then(|value| u16::try_from(value).ok());

    Ok(LoomSeal {
        tx_signature: row.try_get("signature")?,
        writer: row.try_get("wallet")?,
        loom_id: row.try_get("loom_asset")?,
        session_hash: row.try_get("session_hash")?,
        network: row.try_get("network")?,
        utc_day: optional_i64_column(row, "utc_day"),
        slot: slot.and_then(|value| u64::try_from(value).ok()),
        block_time: row.try_get("block_time")?,
        created_at: Some(created_at.to_rfc3339()),
        proof_status: optional_string_column(row, "proof_status"),
        proof_hash: optional_string_column(row, "proof_hash"),
        proof_tx_signature: optional_string_column(row, "proof_signature"),
        proof_verifier: optional_string_column(row, "proof_verifier"),
        proof_protocol_version,
        proof_utc_day: optional_i64_column(row, "proof_utc_day"),
        proof_slot: proof_slot.and_then(|value| u64::try_from(value).ok()),
        proof_block_time: optional_i64_column(row, "proof_block_time"),
        proof_created_at: proof_created_at.map(|value| value.to_rfc3339()),
    })
}

fn mobile_verified_seal_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<MobileVerifiedSeal, AppError> {
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let slot: Option<i64> = row.try_get("slot")?;
    let protocol_version: i32 = row.try_get("protocol_version")?;

    Ok(MobileVerifiedSeal {
        tx_signature: row.try_get("signature")?,
        writer: row.try_get("wallet")?,
        session_hash: row.try_get("session_hash")?,
        proof_hash: row.try_get("proof_hash")?,
        verifier: row.try_get("verifier")?,
        protocol_version: u16::try_from(protocol_version).unwrap_or(0),
        network: row.try_get("network")?,
        status: row.try_get("status")?,
        utc_day: optional_i64_column(row, "utc_day"),
        slot: slot.and_then(|value| u64::try_from(value).ok()),
        block_time: row.try_get("block_time")?,
        created_at: created_at.to_rfc3339(),
    })
}

fn helius_webhook_event_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<HeliusWebhookEventReceipt, AppError> {
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let event_count: i32 = row.try_get("event_count")?;

    Ok(HeliusWebhookEventReceipt {
        id: row.try_get("id")?,
        network: row.try_get("network")?,
        source: row.try_get("source")?,
        payload_hash: row.try_get("payload_hash")?,
        signature: row.try_get("signature")?,
        event_count: u32::try_from(event_count).unwrap_or(0),
        created_at: created_at.to_rfc3339(),
    })
}

fn optional_string_column(row: &sqlx::postgres::PgRow, name: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(name).ok().flatten()
}

fn optional_i32_column(row: &sqlx::postgres::PgRow, name: &str) -> Option<i32> {
    row.try_get::<Option<i32>, _>(name).ok().flatten()
}

fn optional_i64_column(row: &sqlx::postgres::PgRow, name: &str) -> Option<i64> {
    row.try_get::<Option<i64>, _>(name).ok().flatten()
}

fn optional_datetime_column(
    row: &sqlx::postgres::PgRow,
    name: &str,
) -> Option<chrono::DateTime<chrono::Utc>> {
    row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(name)
        .ok()
        .flatten()
}

fn mobile_loom_mint_from_row(row: &sqlx::postgres::PgRow) -> Result<MobileLoomMint, AppError> {
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let loom_index: Option<i32> = row.try_get("loom_index")?;

    Ok(MobileLoomMint {
        id: row.try_get("id")?,
        network: row.try_get("network")?,
        wallet: row.try_get("wallet")?,
        loom_asset: row.try_get("loom_asset")?,
        core_collection: row.try_get("core_collection")?,
        signature: row.try_get("signature")?,
        loom_index: loom_index.and_then(|value| u32::try_from(value).ok()),
        mint_mode: row.try_get("mint_mode")?,
        metadata_uri: row.try_get("metadata_uri")?,
        status: row.try_get("status")?,
        created_at: created_at.to_rfc3339(),
    })
}

fn mobile_reflection_job_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<MobileReflectionJob, AppError> {
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;
    let credits_spent: i32 = row.try_get("credits_spent")?;
    let request_json: Option<String> = row.try_get("request_json")?;
    let result_json: Option<String> = row.try_get("result_json")?;

    Ok(MobileReflectionJob {
        id: row.try_get("id")?,
        identity_id: row.try_get("identity_id")?,
        session_hash: row.try_get("session_hash")?,
        processing_type: row.try_get("processing_type")?,
        status: row.try_get("status")?,
        credits_spent: credits_spent.max(0) as u32,
        request: parse_optional_json(request_json)?,
        result: parse_optional_json(result_json)?,
        error: row.try_get("error")?,
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
    })
}

fn parse_optional_json(value: Option<String>) -> Result<Option<Value>, AppError> {
    value
        .map(|text| serde_json::from_str(&text))
        .transpose()
        .map_err(AppError::from)
}

fn validate_identity_id(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest("identityId is required".into()));
    }
    if value.len() > 256 {
        return Err(AppError::BadRequest("identityId is too long".into()));
    }

    Ok(value.to_string())
}

fn validate_short_text(name: &str, value: &str, max_len: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{} is required", name)));
    }
    if value.len() > max_len {
        return Err(AppError::BadRequest(format!("{} is too long", name)));
    }

    Ok(value.to_string())
}

fn validate_public_key(name: &str, value: &str) -> Result<String, AppError> {
    let value = value.trim();
    let bytes = bs58::decode(value)
        .into_vec()
        .map_err(|_| AppError::BadRequest(format!("{} must be a base58 public key", name)))?;
    if bytes.len() != 32 {
        return Err(AppError::BadRequest(format!(
            "{} must decode to a 32-byte public key",
            name
        )));
    }

    Ok(value.to_string())
}

fn validate_signature(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    let bytes = bs58::decode(value)
        .into_vec()
        .map_err(|_| AppError::BadRequest("signature must be base58".into()))?;
    if bytes.len() != 64 {
        return Err(AppError::BadRequest(
            "signature must decode to a 64-byte Solana signature".into(),
        ));
    }

    Ok(value.to_string())
}

fn validate_expected_collection(value: &str) -> Result<(), AppError> {
    if value == core_collection() {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "coreCollection does not match configured Anky Sojourn 9 Looms collection".into(),
        ))
    }
}

fn validate_expected_proof_verifier(value: &str) -> Result<(), AppError> {
    validate_expected_proof_verifier_value(value, &proof_verifier_authority())
}

fn validate_expected_proof_verifier_value(value: &str, expected: &str) -> Result<(), AppError> {
    if value == expected {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "verifier does not match configured Anky Sojourn 9 proof verifier authority".into(),
        ))
    }
}

fn validate_loom_index(value: u32) -> Result<(), AppError> {
    if (1..=MAX_LOOM_INDEX).contains(&value) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "loomIndex must be between 1 and {}",
            MAX_LOOM_INDEX
        )))
    }
}

fn format_loom_number(value: u32) -> String {
    format!("{value:04}")
}

fn validate_status(value: Option<&str>) -> Result<String, AppError> {
    let status = value.unwrap_or("confirmed").trim();
    match status {
        "confirmed" | "finalized" | "processed" | "pending" | "failed" => Ok(status.to_string()),
        _ => Err(AppError::BadRequest("status is invalid".into())),
    }
}

fn validate_verified_seal_status(value: Option<&str>) -> Result<String, AppError> {
    let status = value.unwrap_or("confirmed").trim();
    match status {
        "confirmed" | "finalized" => Ok(status.to_string()),
        _ => Err(AppError::BadRequest(
            "verified seal status must be confirmed or finalized".into(),
        )),
    }
}

fn require_landed_seal_receipt_status(status: &str) -> Result<(), AppError> {
    match status {
        "confirmed" | "finalized" => Ok(()),
        _ => Err(AppError::BadRequest(
            "matching seal receipt must be confirmed or finalized before verified metadata can be recorded".into(),
        )),
    }
}

fn validate_optional_utc_day(value: Option<i64>) -> Result<Option<i64>, AppError> {
    match value {
        Some(day) if day < 0 => Err(AppError::BadRequest("utcDay must be non-negative".into())),
        other => Ok(other),
    }
}

fn resolve_verified_utc_day(
    requested_utc_day: Option<i64>,
    seal_utc_day: Option<i64>,
) -> Result<i64, AppError> {
    if requested_utc_day.is_some() && seal_utc_day.is_some() && requested_utc_day != seal_utc_day {
        return Err(AppError::BadRequest(
            "utcDay does not match the matching seal receipt".into(),
        ));
    }

    requested_utc_day.or(seal_utc_day).ok_or_else(|| {
        AppError::BadRequest(
            "utcDay is required for verified seal metadata when the matching seal receipt has no utcDay".into(),
        )
    })
}

fn require_verified_seal_record_secret(headers: &HeaderMap) -> Result<(), AppError> {
    require_indexer_write_secret(headers, "verified seal metadata")
}

fn require_finalized_seal_record_secret(status: &str, headers: &HeaderMap) -> Result<(), AppError> {
    if status == "finalized" {
        require_indexer_write_secret(headers, "finalized seal metadata")
    } else {
        Ok(())
    }
}

fn require_indexer_write_secret(headers: &HeaderMap, purpose: &str) -> Result<(), AppError> {
    let expected = env_nonempty("ANKY_VERIFIED_SEAL_RECORD_SECRET")
        .or_else(|| env_nonempty("ANKY_INDEXER_WRITE_SECRET"))
        .ok_or_else(|| {
            AppError::Unavailable(format!(
                "{purpose} recording is not configured on this backend"
            ))
        })?;
    require_indexer_write_secret_value(headers, &expected, purpose)
}

fn indexer_write_secret_matches_config(headers: &HeaderMap) -> bool {
    env_nonempty("ANKY_VERIFIED_SEAL_RECORD_SECRET")
        .or_else(|| env_nonempty("ANKY_INDEXER_WRITE_SECRET"))
        .is_some_and(|expected| verified_seal_record_secret_matches(headers, &expected))
}

fn require_indexer_write_secret_value(
    headers: &HeaderMap,
    expected: &str,
    purpose: &str,
) -> Result<(), AppError> {
    if verified_seal_record_secret_matches(headers, expected) {
        Ok(())
    } else {
        Err(AppError::Unauthorized(format!("invalid {purpose} secret")))
    }
}

fn verified_seal_record_secret_matches(headers: &HeaderMap, expected: &str) -> bool {
    let x_indexer_secret = headers
        .get("x-anky-indexer-secret")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if x_indexer_secret == expected {
        return true;
    }

    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    authorization == expected || authorization == format!("Bearer {expected}")
}

fn require_verified_seal_chain_proof() -> bool {
    env_flag("ANKY_REQUIRE_VERIFIED_SEAL_CHAIN_PROOF")
}

async fn verify_verified_seal_account_on_chain(
    writer: &str,
    session_hash: &str,
    utc_day: i64,
    proof_hash: &str,
    verifier: &str,
    protocol_version: u16,
) -> Result<(), AppError> {
    let writer_pubkey = solana_pubkey("wallet", writer)?;
    let seal_program = solana_pubkey("sealProgramId", &seal_program_id())?;
    let session_hash_bytes = decode_hash_bytes(session_hash)?;
    let proof_hash_bytes = decode_hash_bytes(proof_hash)?;
    let verifier_pubkey = solana_pubkey("verifier", verifier)?;
    let writer_bytes = writer_pubkey.to_bytes();
    let verifier_bytes = verifier_pubkey.to_bytes();
    let (verified_seal_pda, _bump) = SolanaPubkey::find_program_address(
        &[
            VERIFIED_SEAL_SEED,
            writer_pubkey.as_ref(),
            session_hash_bytes.as_ref(),
        ],
        &seal_program,
    );
    let account = fetch_solana_account_base64(&verified_seal_pda.to_string()).await?;
    let data_base64 = solana_account_data_base64(&account.data)?;
    let data = BASE64_STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|_| AppError::Unavailable("VerifiedSeal account data is not base64".into()))?;

    verify_verified_seal_account_data(
        &data,
        &writer_bytes,
        &session_hash_bytes,
        utc_day,
        &proof_hash_bytes,
        &verifier_bytes,
        protocol_version,
    )
}

async fn fetch_solana_account_base64(pubkey: &str) -> Result<SolanaAccountValue, AppError> {
    let response = reqwest::Client::new()
        .post(solana_rpc_url())
        .json(&json!({
            "jsonrpc": "2.0",
            "id": "anky-verified-seal-account",
            "method": "getAccountInfo",
            "params": [
                pubkey,
                {
                    "commitment": "finalized",
                    "encoding": "base64"
                }
            ]
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<SolanaAccountRpcResponse>()
        .await?;

    if let Some(error) = response.error {
        return Err(AppError::Unavailable(format!(
            "Solana RPC getAccountInfo failed: {error}"
        )));
    }

    response
        .result
        .and_then(|result| result.value)
        .ok_or_else(|| {
            AppError::BadRequest("matching on-chain VerifiedSeal account not found".into())
        })
}

fn solana_account_data_base64(value: &Value) -> Result<&str, AppError> {
    if let Some(data) = value.as_str() {
        return Ok(data);
    }
    if let Some(data) = value
        .as_array()
        .and_then(|items| items.first())
        .and_then(Value::as_str)
    {
        return Ok(data);
    }

    Err(AppError::Unavailable(
        "Solana RPC account data is not base64 encoded".into(),
    ))
}

fn verify_verified_seal_account_data(
    data: &[u8],
    writer: &[u8; 32],
    session_hash: &[u8; 32],
    utc_day: i64,
    proof_hash: &[u8; 32],
    verifier: &[u8; 32],
    protocol_version: u16,
) -> Result<(), AppError> {
    const DISCRIMINATOR_LEN: usize = 8;
    const PUBKEY_LEN: usize = 32;
    const HASH_LEN: usize = 32;
    const I64_LEN: usize = 8;
    const U16_LEN: usize = 2;
    const MIN_VERIFIED_SEAL_LEN: usize = DISCRIMINATOR_LEN
        + PUBKEY_LEN
        + HASH_LEN
        + I64_LEN
        + HASH_LEN
        + PUBKEY_LEN
        + U16_LEN
        + I64_LEN;

    if data.len() < MIN_VERIFIED_SEAL_LEN {
        return Err(AppError::BadRequest(
            "on-chain VerifiedSeal account is truncated".into(),
        ));
    }
    if data[..DISCRIMINATOR_LEN] != anchor_discriminator("account:VerifiedSeal") {
        return Err(AppError::BadRequest(
            "on-chain account is not a VerifiedSeal".into(),
        ));
    }

    let mut offset = DISCRIMINATOR_LEN;
    let account_writer = read_fixed_32(data, &mut offset)?;
    let account_session_hash = read_fixed_32(data, &mut offset)?;
    let account_utc_day = read_i64_le(data, &mut offset)?;
    let account_proof_hash = read_fixed_32(data, &mut offset)?;
    let account_verifier = read_fixed_32(data, &mut offset)?;
    let account_protocol_version = read_u16_le(data, &mut offset)?;

    if account_writer != *writer
        || account_session_hash != *session_hash
        || account_utc_day != utc_day
        || account_proof_hash != *proof_hash
        || account_verifier != *verifier
        || account_protocol_version != protocol_version
    {
        return Err(AppError::BadRequest(
            "on-chain VerifiedSeal account does not match submitted metadata".into(),
        ));
    }

    Ok(())
}

fn read_fixed_32(data: &[u8], offset: &mut usize) -> Result<[u8; 32], AppError> {
    let end = offset.checked_add(32).ok_or_else(|| {
        AppError::BadRequest("on-chain VerifiedSeal account offset overflow".into())
    })?;
    let bytes = data
        .get(*offset..end)
        .ok_or_else(|| AppError::BadRequest("on-chain VerifiedSeal account is truncated".into()))?;
    *offset = end;
    let mut fixed = [0u8; 32];
    fixed.copy_from_slice(bytes);

    Ok(fixed)
}

fn read_i64_le(data: &[u8], offset: &mut usize) -> Result<i64, AppError> {
    let end = offset.checked_add(8).ok_or_else(|| {
        AppError::BadRequest("on-chain VerifiedSeal account offset overflow".into())
    })?;
    let bytes = data
        .get(*offset..end)
        .ok_or_else(|| AppError::BadRequest("on-chain VerifiedSeal account is truncated".into()))?;
    *offset = end;

    Ok(i64::from_le_bytes(bytes.try_into().map_err(|_| {
        AppError::BadRequest("on-chain VerifiedSeal account has invalid i64 field".into())
    })?))
}

fn read_u16_le(data: &[u8], offset: &mut usize) -> Result<u16, AppError> {
    let end = offset.checked_add(2).ok_or_else(|| {
        AppError::BadRequest("on-chain VerifiedSeal account offset overflow".into())
    })?;
    let bytes = data
        .get(*offset..end)
        .ok_or_else(|| AppError::BadRequest("on-chain VerifiedSeal account is truncated".into()))?;
    *offset = end;

    Ok(u16::from_le_bytes(bytes.try_into().map_err(|_| {
        AppError::BadRequest("on-chain VerifiedSeal account has invalid u16 field".into())
    })?))
}

fn anchor_discriminator(preimage: &str) -> [u8; 8] {
    let hash = Sha256::digest(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash[..8]);

    discriminator
}

fn decode_hash_bytes(value: &str) -> Result<[u8; 32], AppError> {
    let bytes = hex::decode(value)
        .map_err(|_| AppError::BadRequest("hash must be lowercase hex".into()))?;
    if bytes.len() != 32 {
        return Err(AppError::BadRequest("hash must be 32 bytes".into()));
    }
    let mut fixed = [0u8; 32];
    fixed.copy_from_slice(&bytes);

    Ok(fixed)
}

fn validate_public_webhook_payload(value: &Value) -> Result<(), AppError> {
    if let Some(field) = find_private_webhook_field(value) {
        return Err(AppError::BadRequest(format!(
            "Helius webhook payload must not contain private .anky field `{field}`"
        )));
    }
    if contains_anky_plaintext_value(value) {
        return Err(AppError::BadRequest(
            "Helius webhook payload must not contain .anky plaintext values".into(),
        ));
    }

    Ok(())
}

fn find_private_webhook_field(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => items.iter().find_map(find_private_webhook_field),
        Value::Object(object) => {
            for (key, child) in object {
                if private_webhook_key_name(key) {
                    return Some(key.clone());
                }
                if let Some(field) = find_private_webhook_field(child) {
                    return Some(field);
                }
            }

            None
        }
        _ => None,
    }
}

fn contains_anky_plaintext_value(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(contains_anky_plaintext_value),
        Value::Object(object) => object.values().any(contains_anky_plaintext_value),
        Value::String(text) => looks_like_complete_anky_plaintext(text),
        _ => false,
    }
}

fn looks_like_complete_anky_plaintext(value: &str) -> bool {
    value.contains('\n')
        && value.contains("8000")
        && (validate_closed_anky(value).is_ok() || looks_like_legacy_literal_space_anky(value))
}

fn looks_like_legacy_literal_space_anky(value: &str) -> bool {
    if value.is_empty()
        || value.starts_with('\u{feff}')
        || value.contains('\r')
        || !value.ends_with("\n8000")
        || value.matches("\n8000").count() != 1
    {
        return false;
    }

    let mut lines = value.split('\n');
    let Some(first) = lines.next() else {
        return false;
    };
    if !capture_line_has_valid_time_and_character(first, false) {
        return false;
    }

    for line in lines {
        if line == "8000" {
            return true;
        }
        if !capture_line_has_valid_time_and_character(line, true) {
            return false;
        }
    }

    false
}

fn capture_line_has_valid_time_and_character(line: &str, delta_line: bool) -> bool {
    let Ok((time, character)) = split_capture_line(line) else {
        return false;
    };
    let time_ok = if delta_line {
        time.len() == 4
            && time.chars().all(|ch| ch.is_ascii_digit())
            && time
                .parse::<u16>()
                .map(|delta_ms| delta_ms <= 7_999)
                .unwrap_or(false)
    } else {
        time.parse::<u64>().is_ok()
    };

    time_ok && (character == " " || is_accepted_anky_character(character))
}

fn private_webhook_key_name(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase().replace(['_', '-'], "");
    matches!(
        normalized.as_str(),
        "anky"
            | "rawanky"
            | "plainanky"
            | "ankyplaintext"
            | "ankytext"
            | "ankycontent"
            | "writingplaintext"
            | "plaintext"
            | "sp1witness"
            | "proofwitness"
            | "privatewitness"
            | "witness"
            | "privateinput"
            | "privateinputs"
    )
}

fn count_helius_webhook_items(value: &Value) -> u32 {
    match value {
        Value::Array(items) => u32::try_from(items.len().max(1)).unwrap_or(u32::MAX),
        Value::Object(object) => object
            .get("transactions")
            .and_then(Value::as_array)
            .map(|items| u32::try_from(items.len().max(1)).unwrap_or(u32::MAX))
            .unwrap_or(1),
        _ => 1,
    }
}

fn collect_public_webhook_signatures(value: &Value) -> Vec<String> {
    let mut signatures = Vec::new();
    collect_public_webhook_signatures_into(value, &mut signatures);
    signatures.sort();
    signatures.dedup();
    signatures
}

fn collect_public_webhook_signatures_into(value: &Value, signatures: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_public_webhook_signatures_into(item, signatures);
            }
        }
        Value::Object(object) => {
            for (key, child) in object {
                if matches!(
                    key.as_str(),
                    "signature" | "txSignature" | "transactionSignature"
                ) {
                    if let Some(signature) = child.as_str() {
                        if validate_signature(signature).is_ok() {
                            signatures.push(signature.trim().to_string());
                        }
                    }
                }
                collect_public_webhook_signatures_into(child, signatures);
            }
        }
        _ => {}
    }
}

fn normalize_hash(value: &str) -> Result<String, AppError> {
    validate_hash(value)?;
    Ok(value.trim().to_ascii_lowercase())
}

fn hash_invite_code(value: &str) -> String {
    hash_hex(value.trim().to_ascii_lowercase().as_bytes())
}

fn invite_code_is_allowed(value: &str) -> bool {
    let normalized = value.trim();
    if normalized.is_empty() {
        return false;
    }

    let direct = env_nonempty("ANKY_DEV_INVITE_CODE")
        .map(|expected| expected == normalized)
        .unwrap_or(false);
    if direct {
        return true;
    }

    env_nonempty("ANKY_INVITE_CODES")
        .map(|codes| {
            codes
                .split(',')
                .map(str::trim)
                .any(|candidate| !candidate.is_empty() && candidate == normalized)
        })
        .unwrap_or(false)
}

fn sign_mint_authorization(
    authorization_id: &str,
    wallet: &str,
    payer: &str,
    collection: &str,
    loom_index: u32,
    mode: &str,
    allowed: bool,
    expires_at: i64,
) -> String {
    let secret = env_nonempty("ANKY_MINT_AUTH_SECRET")
        .or_else(|| env_nonempty("ANKY_PROCESSING_RECEIPT_SECRET"))
        .unwrap_or_else(|| DEV_RECEIPT_SECRET.to_string());

    hash_hex(
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}",
            secret,
            authorization_id,
            wallet,
            payer,
            collection,
            loom_index,
            mode,
            allowed,
            expires_at
        )
        .as_bytes(),
    )
}

fn validate_processing_ticket_request(req: &CreateProcessingTicketRequest) -> Result<(), AppError> {
    if req.estimated_entry_count == 0 {
        return Err(AppError::BadRequest(
            "estimatedEntryCount must be greater than zero".into(),
        ));
    }

    if req.session_hashes.len() != req.estimated_entry_count {
        return Err(AppError::BadRequest(
            "estimatedEntryCount must match sessionHashes length".into(),
        ));
    }

    for hash in &req.session_hashes {
        validate_hash(hash)?;
    }

    Ok(())
}

fn validate_receipt(receipt: &CreditReceipt) -> Result<(), AppError> {
    if receipt.receipt_version != 1 {
        return Err(AppError::BadRequest("unsupported receiptVersion".into()));
    }

    if receipt.ticket_id.trim().is_empty() {
        return Err(AppError::BadRequest("ticketId is required".into()));
    }

    if receipt.credits_spent != receipt.processing_type.credit_cost() {
        return Err(AppError::BadRequest(
            "creditsSpent does not match processingType".into(),
        ));
    }

    if receipt.expires_at <= receipt.issued_at {
        return Err(AppError::BadRequest("receipt expiry is invalid".into()));
    }

    if chrono::Utc::now().timestamp_millis() > receipt.expires_at {
        return Err(AppError::BadRequest("processing receipt expired".into()));
    }

    Ok(())
}

fn validate_carpet(carpet: &AnkyCarpet) -> Result<(), AppError> {
    if carpet.carpet_version != 1 {
        return Err(AppError::BadRequest("unsupported carpetVersion".into()));
    }

    if carpet.created_at < 0 {
        return Err(AppError::BadRequest("carpet createdAt is invalid".into()));
    }

    if carpet.entries.is_empty() {
        return Err(AppError::BadRequest(
            "carpet must include at least one entry".into(),
        ));
    }

    for entry in &carpet.entries {
        validate_hash(&entry.session_hash)?;

        let computed_hash = hash_hex(entry.anky.as_bytes());
        if computed_hash != entry.session_hash {
            return Err(AppError::BadRequest(
                "carpet entry hash does not match its .anky bytes".into(),
            ));
        }

        validate_closed_anky(&entry.anky)?;
    }

    Ok(())
}

fn validate_closed_anky(anky: &str) -> Result<(), AppError> {
    if anky.as_bytes().starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(AppError::BadRequest(
            ".anky file must not include a BOM".into(),
        ));
    }

    if anky.contains("\r\n") || anky.contains('\r') {
        return Err(AppError::BadRequest(
            ".anky file must use LF line endings".into(),
        ));
    }

    if !anky.ends_with("\n8000") {
        return Err(AppError::BadRequest(
            ".anky file must end with terminal 8000".into(),
        ));
    }

    if anky.matches("\n8000").count() != 1 {
        return Err(AppError::BadRequest(
            ".anky file must have exactly one terminal 8000 line".into(),
        ));
    }

    let mut lines = anky.split('\n');
    let first = lines
        .next()
        .ok_or_else(|| AppError::BadRequest(".anky file is empty".into()))?;
    validate_epoch_line(first)?;

    for line in lines {
        if line == "8000" {
            break;
        }

        validate_delta_line(line)?;
    }

    Ok(())
}

fn validate_epoch_line(line: &str) -> Result<(), AppError> {
    let (epoch, character) = split_capture_line(line)?;

    if epoch.parse::<u64>().is_err() {
        return Err(AppError::BadRequest(
            ".anky first line must start with Unix epoch milliseconds".into(),
        ));
    }

    if !is_accepted_anky_character(character) {
        return Err(AppError::BadRequest(
            ".anky first line must include one accepted character".into(),
        ));
    }

    Ok(())
}

fn validate_delta_line(line: &str) -> Result<(), AppError> {
    let (delta, character) = split_capture_line(line)?;

    if delta.len() != 4 || !delta.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(AppError::BadRequest(
            ".anky delta lines must start with a zero-padded 4 digit delta".into(),
        ));
    }

    let delta_ms = delta
        .parse::<u16>()
        .map_err(|_| AppError::BadRequest(".anky delta line is invalid".into()))?;
    if delta_ms > 7_999 {
        return Err(AppError::BadRequest(
            ".anky delta lines must be capped at 7999".into(),
        ));
    }

    if !is_accepted_anky_character(character) {
        return Err(AppError::BadRequest(
            ".anky delta line must include one accepted character".into(),
        ));
    }

    Ok(())
}

fn split_capture_line(line: &str) -> Result<(&str, &str), AppError> {
    let Some((time, character)) = line.split_once(' ') else {
        return Err(AppError::BadRequest(
            ".anky capture lines must contain time, separator, and character".into(),
        ));
    };

    Ok((time, character))
}

fn validate_hash(hash: &str) -> Result<(), AppError> {
    if hash.len() == 64 && hash.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Ok(());
    }

    Err(AppError::BadRequest(
        "session hash must be a 32-byte hex string".into(),
    ))
}

fn is_accepted_anky_character(value: &str) -> bool {
    if value == "SPACE" {
        return true;
    }
    if value == " " {
        return false;
    }

    let mut chars = value.chars();
    let Some(character) = chars.next() else {
        return false;
    };

    if chars.next().is_some() {
        return false;
    }

    let codepoint = character as u32;
    codepoint > 31 && codepoint != 127
}

fn reconstruct_closed_anky_text(anky: &str) -> Result<String, AppError> {
    let mut text = String::new();

    for line in anky.split('\n') {
        if line == "8000" {
            break;
        }

        let (_, token) = split_capture_line(line)?;

        if token == "SPACE" {
            text.push(' ');
        } else {
            text.push_str(token);
        }
    }

    Ok(text)
}

async fn build_mobile_reflection_artifacts(
    state: &AppState,
    processing_type: ProcessingType,
    session_hash: &str,
    writing_text: &str,
) -> Result<Vec<Value>, AppError> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let _drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });
    let (full_text, _input_tokens, _output_tokens, model, provider) =
        crate::services::claude::stream_title_and_reflection_best(
            &state.config,
            writing_text,
            tx,
            None,
        )
        .await
        .map_err(|error| {
            tracing::warn!(
                session_hash = %session_hash,
                processing_type = processing_type.as_str(),
                error = %error,
                "mobile reflection provider unavailable"
            );
            AppError::Unavailable("reflection provider unavailable".into())
        })?;
    let (title, reflection) = crate::services::claude::parse_title_reflection(&full_text);
    let title = if title.trim().is_empty() {
        "reflection".to_string()
    } else {
        title
    };
    let markdown = if reflection.trim().is_empty() {
        full_text.trim().to_string()
    } else {
        reflection
    };

    let artifacts = vec![
        json!({
            "kind": "title",
            "sessionHash": session_hash,
            "title": title,
        }),
        json!({
            "kind": "reflection",
            "sessionHash": session_hash,
            "markdown": markdown,
            "provider": provider,
            "model": model,
        }),
    ];

    if processing_type == ProcessingType::FullAnky {
        tracing::info!(
            session_hash = %session_hash,
            "mobile full_anky returned reflection artifacts; image generation is not wired here"
        );
    }

    Ok(artifacts)
}

fn build_dev_artifacts(carpet: &AnkyCarpet, carpet_hash: &str) -> Result<Vec<Value>, AppError> {
    match carpet.purpose {
        ProcessingType::Reflection => Ok(carpet
            .entries
            .iter()
            .map(|entry| reflection_artifact(&entry.session_hash))
            .collect()),
        ProcessingType::Image => Ok(carpet
            .entries
            .iter()
            .map(|entry| image_artifact(&entry.session_hash))
            .collect()),
        ProcessingType::FullAnky => {
            let mut artifacts = Vec::with_capacity(carpet.entries.len() * 3);
            for entry in &carpet.entries {
                artifacts.push(title_artifact(&entry.session_hash));
                artifacts.push(reflection_artifact(&entry.session_hash));
                artifacts.push(image_artifact(&entry.session_hash));
            }
            Ok(artifacts)
        }
        ProcessingType::DeepMirror => Ok(vec![json!({
            "kind": "deep_mirror",
            "carpetHash": carpet_hash,
            "markdown": dev_markdown("deep mirror", carpet.entries.len(), carpet_hash),
        })]),
        ProcessingType::FullSojournArchive => Ok(vec![json!({
            "kind": "full_sojourn_archive",
            "carpetHash": carpet_hash,
            "markdown": dev_markdown("full sojourn archive", carpet.entries.len(), carpet_hash),
            "summaryJson": {
                "mode": "dev_placeholder",
                "entryCount": carpet.entries.len(),
                "carpetHash": carpet_hash,
            },
        })]),
    }
}

fn title_artifact(session_hash: &str) -> Value {
    json!({
        "kind": "title",
        "sessionHash": session_hash,
        "title": format!("Anky {}", &session_hash[..8]),
    })
}

fn reflection_artifact(session_hash: &str) -> Value {
    json!({
        "kind": "reflection",
        "sessionHash": session_hash,
        "markdown": format!(
            "# reflection\n\nDev placeholder for `{}`. The backend verified the .anky bytes and returned a local sidecar artifact; no canonical archive was created.",
            &session_hash[..8]
        ),
    })
}

fn image_artifact(session_hash: &str) -> Value {
    json!({
        "kind": "image",
        "sessionHash": session_hash,
        "imageBase64": PLACEHOLDER_IMAGE_PNG_BASE64,
        "mimeType": "image/png",
    })
}

fn dev_markdown(label: &str, entry_count: usize, carpet_hash: &str) -> String {
    format!(
        "# {}\n\nDev placeholder for {} verified .anky file(s).\n\ncarpet hash: `{}`\n\nNo canonical archive was created on the backend.",
        label, entry_count, carpet_hash
    )
}

fn hash_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn sign_receipt_fields(
    secret: &str,
    ticket_id: &str,
    processing_type: ProcessingType,
    credits_spent: u32,
    credits_remaining: u32,
    issued_at: i64,
    expires_at: i64,
    nonce: &str,
) -> String {
    hash_hex(
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            secret,
            ticket_id,
            processing_type.as_str(),
            credits_spent,
            credits_remaining,
            issued_at,
            expires_at,
            nonce
        )
        .as_bytes(),
    )
}

fn receipt_secret() -> Result<String, AppError> {
    if let Some(secret) = env_nonempty("ANKY_PROCESSING_RECEIPT_SECRET") {
        return Ok(secret);
    }

    if dev_plaintext_processing_allowed() {
        return Ok(DEV_RECEIPT_SECRET.to_string());
    }

    Err(AppError::Unavailable(
        "credit receipt signing is not configured".into(),
    ))
}

fn dev_credit_balance() -> u32 {
    env_nonempty("ANKY_DEV_CREDITS_REMAINING")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_else(|| {
            if dev_plaintext_processing_allowed() {
                88
            } else {
                0
            }
        })
}

fn dev_plaintext_processing_allowed() -> bool {
    env_flag("ALLOW_DEV_PLAINTEXT_PROCESSING") || env_flag("ANKY_ALLOW_DEV_PLAINTEXT_PROCESSING")
}

fn solana_cluster() -> String {
    match env_nonempty("ANKY_SOLANA_CLUSTER").as_deref() {
        Some("mainnet-beta") => "mainnet-beta".to_string(),
        _ => DEFAULT_SOLANA_CLUSTER.to_string(),
    }
}

fn solana_rpc_url() -> String {
    resolve_server_solana_rpc_url(
        &solana_cluster(),
        env_nonempty("ANKY_SOLANA_RPC_URL"),
        env_nonempty("EXPO_PUBLIC_SOLANA_RPC_URL"),
    )
}

fn public_solana_rpc_url() -> String {
    resolve_public_solana_rpc_url(
        &solana_cluster(),
        env_nonempty("ANKY_PUBLIC_SOLANA_RPC_URL"),
        env_nonempty("EXPO_PUBLIC_SOLANA_RPC_URL"),
    )
}

fn resolve_server_solana_rpc_url(
    cluster: &str,
    server_rpc_url: Option<String>,
    expo_public_rpc_url: Option<String>,
) -> String {
    server_rpc_url
        .or(expo_public_rpc_url)
        .unwrap_or_else(|| default_solana_rpc_url_for_cluster(cluster))
}

fn resolve_public_solana_rpc_url(
    cluster: &str,
    public_rpc_url: Option<String>,
    expo_public_rpc_url: Option<String>,
) -> String {
    public_rpc_url
        .or(expo_public_rpc_url)
        .unwrap_or_else(|| default_solana_rpc_url_for_cluster(cluster))
}

fn default_solana_rpc_url() -> String {
    default_solana_rpc_url_for_cluster(&solana_cluster())
}

fn default_solana_rpc_url_for_cluster(cluster: &str) -> String {
    if cluster == "mainnet-beta" {
        DEFAULT_MAINNET_SOLANA_RPC_URL.to_string()
    } else {
        DEFAULT_SOLANA_RPC_URL.to_string()
    }
}

fn core_program_id() -> String {
    env_nonempty("ANKY_CORE_PROGRAM_ID")
        .or_else(|| env_nonempty("EXPO_PUBLIC_ANKY_CORE_PROGRAM_ID"))
        .unwrap_or_else(|| DEFAULT_CORE_PROGRAM_ID.to_string())
}

fn core_collection() -> String {
    env_nonempty("ANKY_CORE_COLLECTION")
        .or_else(|| env_nonempty("EXPO_PUBLIC_ANKY_CORE_COLLECTION"))
        .unwrap_or_else(|| DEFAULT_CORE_COLLECTION.to_string())
}

fn seal_program_id() -> String {
    env_nonempty("ANKY_SEAL_PROGRAM_ID")
        .or_else(|| env_nonempty("EXPO_PUBLIC_ANKY_SEAL_PROGRAM_ID"))
        .unwrap_or_else(|| DEFAULT_SEAL_PROGRAM_ID.to_string())
}

fn proof_verifier_authority() -> String {
    env_nonempty("ANKY_PROOF_VERIFIER_AUTHORITY")
        .or_else(|| env_nonempty("EXPO_PUBLIC_ANKY_PROOF_VERIFIER_AUTHORITY"))
        .unwrap_or_else(|| DEFAULT_PROOF_VERIFIER_AUTHORITY.to_string())
}

fn collection_uri() -> String {
    env_nonempty("ANKY_COLLECTION_METADATA_URI").unwrap_or_else(|| {
        if solana_cluster() == "mainnet-beta" {
            DEFAULT_MAINNET_COLLECTION_URI.to_string()
        } else {
            DEFAULT_COLLECTION_URI.to_string()
        }
    })
}

fn loom_metadata_base_url() -> String {
    env_nonempty("ANKY_LOOM_METADATA_BASE_URL").unwrap_or_else(|| {
        if solana_cluster() == "mainnet-beta" {
            DEFAULT_MAINNET_LOOM_METADATA_BASE_URL.to_string()
        } else {
            DEFAULT_LOOM_METADATA_BASE_URL.to_string()
        }
    })
}

fn seal_verification_label() -> String {
    if solana_cluster() == "mainnet-beta" {
        "mainnet_core_base_account_verification".to_string()
    } else {
        "devnet_core_base_account_verification".to_string()
    }
}

fn sojourn_9_program_id() -> String {
    env_nonempty("ANKY_SOJOURN9_PROGRAM_ID")
        .or_else(|| env_nonempty("SOLANA_PROGRAM_ID"))
        .unwrap_or_else(|| DEFAULT_SOJOURN_9_PROGRAM_ID.to_string())
}

fn initial_mobile_credits() -> u32 {
    env_nonempty("ANKY_INITIAL_MOBILE_CREDITS")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(DEFAULT_INITIAL_MOBILE_CREDITS)
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processing_type_costs_match_mobile_contract() {
        assert_eq!(ProcessingType::Reflection.credit_cost(), 1);
        assert_eq!(ProcessingType::Image.credit_cost(), 3);
        assert_eq!(ProcessingType::FullAnky.credit_cost(), 5);
        assert_eq!(ProcessingType::DeepMirror.credit_cost(), 8);
        assert_eq!(ProcessingType::FullSojournArchive.credit_cost(), 88);
    }

    #[tokio::test]
    async fn app_config_includes_proof_verifier_authority() {
        let Json(config) = get_config().await;
        let value = serde_json::to_value(config).unwrap();

        assert_eq!(
            value["solana"]["proofVerifierAuthority"],
            proof_verifier_authority()
        );
    }

    #[test]
    fn mobile_solana_config_includes_proof_verifier_authority() {
        let value = serde_json::to_value(mobile_solana_config()).unwrap();

        assert_eq!(value["proofVerifierAuthority"], proof_verifier_authority());
    }

    #[test]
    fn solana_rpc_resolvers_keep_private_rpc_out_of_public_config() {
        let server_rpc = resolve_server_solana_rpc_url(
            "devnet",
            Some("https://private-helius.example/?api-key=secret".to_string()),
            Some("https://public-expo.example".to_string()),
        );
        let public_rpc = resolve_public_solana_rpc_url(
            "devnet",
            Some("https://public-mobile.example".to_string()),
            Some("https://public-expo.example".to_string()),
        );

        assert_eq!(server_rpc, "https://private-helius.example/?api-key=secret");
        assert_eq!(public_rpc, "https://public-mobile.example");
    }

    #[test]
    fn public_solana_rpc_resolver_falls_back_to_expo_then_default() {
        assert_eq!(
            resolve_public_solana_rpc_url(
                "devnet",
                None,
                Some("https://public-expo.example".to_string())
            ),
            "https://public-expo.example"
        );
        assert_eq!(
            resolve_public_solana_rpc_url("devnet", None, None),
            DEFAULT_SOLANA_RPC_URL
        );
        assert_eq!(
            resolve_public_solana_rpc_url("mainnet-beta", None, None),
            DEFAULT_MAINNET_SOLANA_RPC_URL
        );
    }

    #[test]
    fn validate_closed_anky_rejects_literal_space_character() {
        let anky = "1710000000000 a\n0001  \n8000";
        assert!(validate_closed_anky(anky).is_err());
    }

    #[test]
    fn validate_closed_anky_accepts_space_token() {
        let anky = "1710000000000 a\n0001 SPACE\n8000";
        assert!(validate_closed_anky(anky).is_ok());
        assert_eq!(reconstruct_closed_anky_text(anky).unwrap(), "a ");
    }

    #[test]
    fn validate_closed_anky_rejects_multi_character_commits() {
        let anky = "1710000000000 ab\n8000";
        assert!(validate_closed_anky(anky).is_err());
    }

    #[test]
    fn validate_closed_anky_rejects_text_after_terminal_line() {
        let anky = "1710000000000 a\n8000\nextra";
        assert!(validate_closed_anky(anky).is_err());
    }

    #[test]
    fn validate_carpet_verifies_hashes_from_exact_bytes() {
        let anky = "1710000000000 a\n0001 b\n8000";
        let session_hash = hash_hex(anky.as_bytes());
        let carpet = AnkyCarpet {
            carpet_version: 1,
            purpose: ProcessingType::Reflection,
            created_at: 1,
            entries: vec![CarpetEntry {
                session_hash,
                anky: anky.to_string(),
            }],
        };

        assert!(validate_carpet(&carpet).is_ok());
    }

    #[test]
    fn verified_seal_record_secret_requires_matching_header() {
        let mut headers = HeaderMap::new();
        assert!(!verified_seal_record_secret_matches(
            &headers,
            "expected-secret"
        ));

        headers.insert("x-anky-indexer-secret", "wrong-secret".parse().unwrap());
        assert!(!verified_seal_record_secret_matches(
            &headers,
            "expected-secret"
        ));

        headers.insert("x-anky-indexer-secret", "expected-secret".parse().unwrap());
        assert!(verified_seal_record_secret_matches(
            &headers,
            "expected-secret"
        ));

        headers.remove("x-anky-indexer-secret");
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer expected-secret".parse().unwrap(),
        );
        assert!(verified_seal_record_secret_matches(
            &headers,
            "expected-secret"
        ));

        headers.insert(
            axum::http::header::AUTHORIZATION,
            "expected-secret".parse().unwrap(),
        );
        assert!(verified_seal_record_secret_matches(
            &headers,
            "expected-secret"
        ));
    }

    #[test]
    fn verified_seal_record_requires_configured_verifier_authority() {
        assert!(validate_expected_proof_verifier_value(
            DEFAULT_PROOF_VERIFIER_AUTHORITY,
            DEFAULT_PROOF_VERIFIER_AUTHORITY
        )
        .is_ok());
        let error = validate_expected_proof_verifier_value(
            "11111111111111111111111111111111",
            DEFAULT_PROOF_VERIFIER_AUTHORITY,
        )
        .unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)));
        assert!(error.to_string().contains("proof verifier authority"));
    }

    #[test]
    fn utc_day_validator_rejects_negative_values() {
        assert_eq!(validate_optional_utc_day(None).unwrap(), None);
        assert_eq!(
            validate_optional_utc_day(Some(19_999)).unwrap(),
            Some(19_999)
        );

        let error = validate_optional_utc_day(Some(-1)).unwrap_err();
        assert!(matches!(error, AppError::BadRequest(_)));
        assert!(error.to_string().contains("utcDay"));
    }

    #[test]
    fn verified_seal_status_requires_landed_receipt_status() {
        assert_eq!(validate_verified_seal_status(None).unwrap(), "confirmed");
        assert_eq!(
            validate_verified_seal_status(Some("finalized")).unwrap(),
            "finalized"
        );

        for status in ["pending", "processed", "failed"] {
            let error = validate_verified_seal_status(Some(status)).unwrap_err();
            assert!(matches!(error, AppError::BadRequest(_)));
            assert!(error.to_string().contains("confirmed or finalized"));
        }
    }

    #[test]
    fn public_seal_status_allows_mobile_receipt_lifecycle_states() {
        for status in ["confirmed", "finalized", "processed", "pending", "failed"] {
            assert_eq!(validate_status(Some(status)).unwrap(), status);
        }
    }

    #[test]
    fn finalized_public_seal_metadata_requires_indexer_secret() {
        let mut headers = HeaderMap::new();
        assert!(require_finalized_seal_record_secret("confirmed", &headers).is_ok());
        assert!(require_finalized_seal_record_secret("pending", &headers).is_ok());

        let error = require_indexer_write_secret_value(
            &headers,
            "expected-secret",
            "finalized seal metadata",
        )
        .unwrap_err();
        assert!(matches!(error, AppError::Unauthorized(_)));
        assert!(error.to_string().contains("finalized seal metadata"));

        headers.insert("x-anky-indexer-secret", "expected-secret".parse().unwrap());
        assert!(require_indexer_write_secret_value(
            &headers,
            "expected-secret",
            "finalized seal metadata",
        )
        .is_ok());
    }

    #[test]
    fn finalized_public_seal_conflict_update_is_sticky() {
        let source = include_str!("mobile_sojourn.rs");

        assert!(source.contains("WHERE mobile_seal_receipts.status <> 'finalized'"));
        assert!(source.contains("OR ($12 AND EXCLUDED.status = 'finalized')"));
        assert!(source.contains("finalized seal metadata is immutable"));
    }

    #[test]
    fn mobile_seal_score_uses_score_v1_formula_with_streak_runs() {
        let score = build_mobile_seal_score(
            "11111111111111111111111111111111".to_string(),
            "devnet".to_string(),
            DEFAULT_PROOF_VERIFIER_AUTHORITY.to_string(),
            vec![
                20_001, 20_000, 20_002, 20_003, 20_004, 20_005, 20_006, 20_007, 20_008, 20_009,
                20_010, 20_011, 20_012, 20_013, 20_020, 20_021, 20_022, 20_023, 20_024, 20_025,
                20_026,
            ],
            vec![20_000, 20_000, 20_013, 20_026],
        );

        assert_eq!(score.unique_seal_days, 21);
        assert_eq!(score.verified_seal_days, 3);
        assert_eq!(score.streak_bonus, 6);
        assert_eq!(score.score, 30);
        assert_eq!(score.sealed_days.first().copied(), Some(20_000));
        assert_eq!(score.sealed_days.last().copied(), Some(20_026));
        assert!(score.finalized_only);
        assert!(score.formula.contains("unique_seal_days"));
    }

    #[test]
    fn mobile_seal_score_drops_verified_days_without_matching_sealed_day() {
        let score = build_mobile_seal_score(
            "11111111111111111111111111111111".to_string(),
            "devnet".to_string(),
            DEFAULT_PROOF_VERIFIER_AUTHORITY.to_string(),
            vec![20_001, 20_001, -1],
            vec![20_001, 20_002],
        );

        assert_eq!(score.sealed_days, vec![20_001]);
        assert_eq!(score.verified_days, vec![20_001]);
        assert_eq!(score.score, 2);
    }

    #[test]
    fn verified_metadata_requires_landed_matching_seal_status() {
        assert!(require_landed_seal_receipt_status("confirmed").is_ok());
        assert!(require_landed_seal_receipt_status("finalized").is_ok());

        for status in ["pending", "processed", "failed"] {
            let error = require_landed_seal_receipt_status(status).unwrap_err();
            assert!(matches!(error, AppError::BadRequest(_)));
            assert!(error.to_string().contains("matching seal receipt"));
        }
    }

    #[test]
    fn verified_utc_day_resolution_requires_known_day_and_rejects_mismatch() {
        assert_eq!(
            resolve_verified_utc_day(Some(19_999), Some(19_999)).unwrap(),
            19_999
        );
        assert_eq!(
            resolve_verified_utc_day(Some(19_999), None).unwrap(),
            19_999
        );
        assert_eq!(
            resolve_verified_utc_day(None, Some(19_999)).unwrap(),
            19_999
        );

        let missing = resolve_verified_utc_day(None, None).unwrap_err();
        assert!(matches!(missing, AppError::BadRequest(_)));
        assert!(missing.to_string().contains("utcDay is required"));

        let mismatch = resolve_verified_utc_day(Some(20_000), Some(19_999)).unwrap_err();
        assert!(matches!(mismatch, AppError::BadRequest(_)));
        assert!(mismatch.to_string().contains("utcDay does not match"));
    }

    #[test]
    fn verified_seal_account_data_must_match_submitted_public_metadata() {
        let writer = [1u8; 32];
        let session_hash = [2u8; 32];
        let proof_hash = [3u8; 32];
        let verifier = [4u8; 32];
        let data =
            verified_seal_account_data(writer, session_hash, 20_000, proof_hash, verifier, 1);

        assert!(verify_verified_seal_account_data(
            &data,
            &writer,
            &session_hash,
            20_000,
            &proof_hash,
            &verifier,
            1,
        )
        .is_ok());

        let mismatch = verify_verified_seal_account_data(
            &data,
            &writer,
            &session_hash,
            20_001,
            &proof_hash,
            &verifier,
            1,
        )
        .unwrap_err();
        assert!(matches!(mismatch, AppError::BadRequest(_)));
        assert!(mismatch.to_string().contains("does not match"));
    }

    #[test]
    fn verified_seal_account_data_rejects_wrong_discriminator() {
        let mut data =
            verified_seal_account_data([1u8; 32], [2u8; 32], 20_000, [3u8; 32], [4u8; 32], 1);
        data[0] ^= 0xff;

        let error = verify_verified_seal_account_data(
            &data, &[1u8; 32], &[2u8; 32], 20_000, &[3u8; 32], &[4u8; 32], 1,
        )
        .unwrap_err();
        assert!(matches!(error, AppError::BadRequest(_)));
        assert!(error.to_string().contains("not a VerifiedSeal"));
    }

    #[test]
    fn helius_webhook_payload_rejects_private_anky_fields() {
        let payload = json!({
            "signature": "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh",
            "accountData": [
                {
                    "rawAnky": "1710000000000 a\n8000"
                }
            ]
        });

        let error = validate_public_webhook_payload(&payload).unwrap_err();
        assert!(matches!(error, AppError::BadRequest(_)));
        assert!(error.to_string().contains("rawAnky"));
    }

    #[test]
    fn helius_webhook_payload_rejects_anky_plaintext_values_under_generic_keys() {
        let payload = json!({
            "signature": "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh",
            "instructions": [
                {
                    "programId": seal_program_id(),
                    "memo": "1710000000000 a\n8000"
                }
            ]
        });

        let error = validate_public_webhook_payload(&payload).unwrap_err();
        assert!(matches!(error, AppError::BadRequest(_)));
        assert!(error.to_string().contains("plaintext values"));
    }

    #[test]
    fn helius_webhook_payload_rejects_legacy_literal_space_anky_plaintext_values() {
        let payload = json!({
            "signature": "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh",
            "instructions": [
                {
                    "programId": seal_program_id(),
                    "memo": "1710000000000 a\n0001  \n8000"
                }
            ]
        });

        let error = validate_public_webhook_payload(&payload).unwrap_err();
        assert!(matches!(error, AppError::BadRequest(_)));
        assert!(error.to_string().contains("plaintext values"));
    }

    #[test]
    fn helius_webhook_payload_counts_public_items_and_dedupes_signatures() {
        let signature = "2hntvJaJzRkFWt3hTa7Q9oiGyVsTpjMwmzY8WcN52UDMsTyMuzKUtcEhupAe7BcZGeq49dFBhhgoYgeZ79m53sNh";
        let payload = json!([
            {
                "signature": signature,
                "description": "public Anchor event payload"
            },
            {
                "txSignature": signature,
                "transactionSignature": "not-a-signature"
            }
        ]);

        validate_public_webhook_payload(&payload).unwrap();
        assert_eq!(count_helius_webhook_items(&payload), 2);
        assert_eq!(
            collect_public_webhook_signatures(&payload),
            vec![signature.to_string()]
        );
    }

    #[test]
    fn validate_carpet_rejects_mismatched_hashes() {
        let carpet = AnkyCarpet {
            carpet_version: 1,
            purpose: ProcessingType::Reflection,
            created_at: 1,
            entries: vec![CarpetEntry {
                session_hash: "a".repeat(64),
                anky: "1710000000000 a\n8000".to_string(),
            }],
        };

        assert!(validate_carpet(&carpet).is_err());
    }

    #[test]
    fn mobile_thread_accepts_valid_fragment_payload() {
        let req = validate_mobile_thread_payload(sample_mobile_thread_payload("fragment")).unwrap();
        let provider_messages = build_mobile_thread_provider_messages(&req);
        let response = serde_json::to_value(mobile_thread_response(
            "i am here with this unfinished thread. what was trying to arrive?".to_string(),
        ))
        .unwrap();

        assert_eq!(req.session_hash, "a".repeat(64));
        assert_eq!(req.mode, MobileThreadMode::Fragment);
        assert_eq!(provider_messages[0].0, "assistant");
        assert_eq!(provider_messages.last().unwrap().0, "user");
        assert_eq!(response["message"]["role"], "anky");
        assert!(response["message"]["createdAt"].as_str().unwrap().len() > 10);
    }

    #[test]
    fn mobile_thread_accepts_valid_complete_payload() {
        let req = validate_mobile_thread_payload(sample_mobile_thread_payload("complete")).unwrap();
        let system = build_mobile_thread_system_prompt(&req);
        let response = serde_json::to_value(mobile_thread_response(
            "the completed thread is still warm. what wants to remain with you?".to_string(),
        ))
        .unwrap();

        assert_eq!(req.mode, MobileThreadMode::Complete);
        assert!(system.contains("this is a complete anky"));
        assert_eq!(response["message"]["role"], "anky");
    }

    #[test]
    fn mobile_thread_rejects_invalid_mode() {
        let mut payload = sample_mobile_thread_payload("fragment");
        payload["mode"] = json!("chatbot");

        let error = validate_mobile_thread_payload(payload).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)));
        assert!(error.to_string().contains("mode is invalid"));
    }

    #[test]
    fn mobile_thread_rejects_missing_user_message_without_echoing_plaintext() {
        let mut payload = sample_mobile_thread_payload("reflection");
        payload.as_object_mut().unwrap().remove("userMessage");

        let error = validate_mobile_thread_payload(payload).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)));
        assert!(error.to_string().contains("userMessage is required"));
        assert!(!error.to_string().contains("private words"));
    }

    #[test]
    fn mobile_thread_safety_response_uses_anky_role() {
        let mut payload = sample_mobile_thread_payload("fragment");
        payload["userMessage"] = json!("i want to die tonight");
        let req = validate_mobile_thread_payload(payload).unwrap();
        let response =
            serde_json::to_value(mobile_thread_response(mobile_thread_safety_response())).unwrap();

        assert!(mobile_thread_needs_immediate_safety_response(&req));
        assert_eq!(response["message"]["role"], "anky");
        assert!(response["message"]["content"]
            .as_str()
            .unwrap()
            .contains("local emergency number"));
    }

    #[tokio::test]
    async fn mobile_thread_provider_failure_returns_safe_error_body() {
        let response = MobileThreadError::ThreadUnavailable.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(value["error"], "thread_unavailable");
        assert_eq!(
            value["message"],
            "anky cannot continue the thread right now."
        );
    }

    fn sample_mobile_thread_payload(mode: &str) -> Value {
        json!({
            "sessionHash": "a".repeat(64),
            "mode": mode,
            "rawAnky": "1710000000000 p\n0001 r\n0002 i\n0003 v\n0004 a\n0005 t\n0006 e\n0007  \n0008 w\n0009 o\n0010 r\n0011 d\n0012 s\n8000",
            "reconstructedText": "private words are trying to become a thread",
            "existingReflection": "the mirror already noticed the doorway.",
            "messages": [
                {
                    "role": "anky",
                    "content": "i am here with the doorway.",
                    "createdAt": "2026-04-30T00:00:00.000Z"
                }
            ],
            "userMessage": "what is still alive here?"
        })
    }

    fn verified_seal_account_data(
        writer: [u8; 32],
        session_hash: [u8; 32],
        utc_day: i64,
        proof_hash: [u8; 32],
        verifier: [u8; 32],
        protocol_version: u16,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&anchor_discriminator("account:VerifiedSeal"));
        data.extend_from_slice(&writer);
        data.extend_from_slice(&session_hash);
        data.extend_from_slice(&utc_day.to_le_bytes());
        data.extend_from_slice(&proof_hash);
        data.extend_from_slice(&verifier);
        data.extend_from_slice(&protocol_version.to_le_bytes());
        data.extend_from_slice(&1_700_000_000i64.to_le_bytes());

        data
    }
}
