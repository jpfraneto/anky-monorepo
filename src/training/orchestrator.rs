use crate::db::queries;
use crate::state::{AppState, GpuStatus};
use crate::training::{dataset, runner};
use anyhow::Result;
use std::path::Path;

/// Run the full training cycle:
/// 1. Set GPU status to Training
/// 2. Prepare dataset (base + new ankys)
/// 3. Run training subprocess
/// 4. Set GPU status back to Idle
/// 5. Send notifications
pub async fn run_training_cycle(state: &AppState) -> Result<()> {
    // Check if GPUs are idle
    {
        let status = state.gpu_status.read().await;
        if *status != GpuStatus::Idle {
            state.emit_log("WARN", "orchestrator", &format!("GPUs not idle ({}), skipping training", status));
            return Ok(());
        }
    }

    state.emit_log("INFO", "orchestrator", "Starting training cycle...");

    // Set GPU status
    {
        let mut status = state.gpu_status.write().await;
        *status = GpuStatus::Training { step: 0, total: 4000 };
    }

    let training_run_id = uuid::Uuid::new_v4().to_string();
    let run_dir = format!("data/training_runs/{}", training_run_id);
    let dataset_out = format!("{}/dataset", run_dir);

    // Prepare dataset
    let base_dataset_str = std::env::var("BASE_DATASET_DIR")
        .unwrap_or_else(|_| "/home/kithkui/Desktop/code/z-image-turbo/files/anky_lora_training/dataset".into());
    let base_dataset = Path::new(&base_dataset_str);
    let generated = Path::new("data/images");

    let (dataset_path, dataset_size) =
        dataset::prepare_dataset(base_dataset, generated, Path::new(&dataset_out))?;

    if dataset_size == 0 {
        state.emit_log("WARN", "orchestrator", "No training data found, aborting");
        let mut status = state.gpu_status.write().await;
        *status = GpuStatus::Idle;
        return Ok(());
    }

    // Record training run
    {
        let db = state.db.lock().await;
        queries::insert_training_run(&db, &training_run_id, "FLUX.1-dev", dataset_size as i32, 4000)?;
    }

    state.emit_log(
        "INFO",
        "orchestrator",
        &format!("Dataset ready: {} pairs. Starting training...", dataset_size),
    );

    let output_dir = format!("{}/output", run_dir);

    // Run training
    match runner::run_training(state, &training_run_id, &dataset_path, &output_dir, 4000).await {
        Ok(lora_path) => {
            // Copy lora weights to stable location
            let stable_path = format!("data/lora_weights/{}.safetensors", training_run_id);
            std::fs::create_dir_all("data/lora_weights")?;
            if Path::new(&lora_path).exists() {
                std::fs::copy(&lora_path, &stable_path)?;
            }

            {
                let db = state.db.lock().await;
                queries::complete_training_run(&db, &training_run_id, &stable_path)?;
            }

            state.emit_log("INFO", "orchestrator", &format!("Training complete! LoRA: {}", stable_path));
        }
        Err(e) => {
            state.emit_log("ERROR", "orchestrator", &format!("Training failed: {}", e));
        }
    }

    // Reset GPU status
    {
        let mut status = state.gpu_status.write().await;
        *status = GpuStatus::Idle;
    }

    // Send notifications
    let signups = {
        let db = state.db.lock().await;
        queries::get_notification_signups(&db)?
    };

    for (email, telegram) in &signups {
        if let Some(email) = email {
            let _ = crate::services::notification::send_email_notification(
                email,
                "Anky is awake!",
                "Training is complete. Anky has evolved. Come write!",
            )
            .await;
        }
        if let Some(chat_id) = telegram {
            let _ = crate::services::notification::send_telegram_notification(
                chat_id,
                "Anky is awake! Training is complete. Come write!",
                None,
            )
            .await;
        }
    }

    state.emit_log("INFO", "orchestrator", "Training cycle complete. Anky is awake.");

    Ok(())
}
