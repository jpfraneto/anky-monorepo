use crate::error::AppError;
use crate::models::{PaymentVerifyRequest, PaymentVerifyResponse};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

pub async fn verify_payment(
    State(state): State<AppState>,
    Json(req): Json<PaymentVerifyRequest>,
) -> Result<Json<PaymentVerifyResponse>, AppError> {
    state.emit_log(
        "INFO",
        "payment",
        &format!("Verifying payment tx: {}...", &req.tx_hash[..10]),
    );

    let result = crate::services::payment::verify_base_transaction(
        &state.config.base_rpc_url,
        &req.tx_hash,
        &state.config.treasury_address,
        &state.config.usdc_address,
        &req.expected_amount,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Payment verification failed: {}", e)))?;

    if result.valid {
        // Update collection payment
        let db = state.db.lock().await;
        crate::db::queries::update_collection_payment(&db, &req.collection_id, &req.tx_hash)?;

        state.emit_log(
            "INFO",
            "payment",
            &format!("Payment verified for collection {}", &req.collection_id[..8]),
        );

        // Start collection generation in background
        drop(db);
        let state_clone = state.clone();
        let collection_id = req.collection_id.clone();
        tokio::spawn(async move {
            // Expand beings and generate
            match crate::pipeline::collection::expand_beings(&state_clone, "").await {
                Ok(beings) => {
                    if let Err(e) = crate::pipeline::collection::generate_collection(
                        &state_clone,
                        &collection_id,
                        &beings,
                    )
                    .await
                    {
                        tracing::error!("Collection generation failed: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Being expansion failed: {}", e);
                }
            }
        });
    }

    Ok(Json(PaymentVerifyResponse {
        valid: result.valid,
        reason: result.reason,
    }))
}
