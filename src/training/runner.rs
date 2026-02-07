use crate::state::AppState;
use anyhow::Result;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Spawn the Python training script and monitor its output.
pub async fn run_training(
    state: &AppState,
    training_run_id: &str,
    dataset_dir: &str,
    output_dir: &str,
    steps: u32,
) -> Result<String> {
    let script_path = "training/train_flux_lora.py";

    state.emit_log(
        "INFO",
        "training",
        &format!(
            "Starting FLUX.1-dev LoRA training: {} steps, dataset: {}",
            steps, dataset_dir
        ),
    );

    let mut child = Command::new("python3")
        .arg(script_path)
        .arg("--dataset_dir")
        .arg(dataset_dir)
        .arg("--output_dir")
        .arg(output_dir)
        .arg("--max_train_steps")
        .arg(steps.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();

    let state_clone = state.clone();
    let run_id = training_run_id.to_string();

    // Read stdout for progress updates
    tokio::spawn(async move {
        while let Ok(Some(line)) = reader.next_line().await {
            // Try to parse JSON progress lines
            if let Ok(progress) = serde_json::from_str::<serde_json::Value>(&line) {
                if let (Some(step), Some(loss)) = (
                    progress.get("step").and_then(|v| v.as_i64()),
                    progress.get("loss").and_then(|v| v.as_f64()),
                ) {
                    let _ = {
                        let db = state_clone.db.lock().await;
                        crate::db::queries::update_training_progress(
                            &db,
                            &run_id,
                            step as i32,
                            loss,
                        )
                    };

                    // Update GPU status
                    {
                        let mut gpu = state_clone.gpu_status.write().await;
                        *gpu = crate::state::GpuStatus::Training {
                            step: step as u32,
                            total: 4000,
                        };
                    }
                }
            }

            state_clone.emit_log("INFO", "training", &line);
        }
    });

    let status = child.wait().await?;

    if !status.success() {
        anyhow::bail!("Training process exited with status: {}", status);
    }

    let lora_path = format!("{}/final/pytorch_lora_weights.safetensors", output_dir);
    Ok(lora_path)
}
