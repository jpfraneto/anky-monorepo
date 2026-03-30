use crate::db::queries;
use crate::services::{apns, claude};
use crate::state::AppState;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Start the daily push notification scheduler (5:30 AM UTC).
pub async fn start_scheduler(state: AppState) -> anyhow::Result<()> {
    let sched = JobScheduler::new().await?;

    let state_clone = state.clone();
    // Run at 5:30 AM UTC every day
    sched
        .add(Job::new_async("0 30 5 * * *", move |_uuid, _l| {
            let state = state_clone.clone();
            Box::pin(async move {
                tracing::info!("Daily push notification job fired");
                state.emit_log(
                    "INFO",
                    "push",
                    "Daily push notification job triggered (5:30 AM UTC)",
                );
                if let Err(e) = send_daily_notifications(&state).await {
                    tracing::error!("Daily push error: {}", e);
                    state.emit_log("ERROR", "push", &format!("Daily push error: {}", e));
                }
            })
        })?)
        .await?;

    sched.start().await?;
    tracing::info!("Push notification scheduler started (daily at 5:30 AM UTC)");

    Ok(())
}

async fn send_daily_notifications(state: &AppState) -> anyhow::Result<()> {
    let apns_client = match apns::build_client(&state.config) {
        Some(c) => c,
        None => {
            tracing::warn!("APNs not configured, skipping daily push");
            return Ok(());
        }
    };

    let bundle_id = state.config.apns_bundle_id.clone();
    if bundle_id.is_empty() {
        tracing::warn!("APNS_BUNDLE_ID not set, skipping daily push");
        return Ok(());
    }

    // Get all users with device tokens + at least one writing session
    let targets = {
        let db = crate::db::conn(&state.db)?;
        queries::get_notification_targets(&db)?
    };

    if targets.is_empty() {
        tracing::info!("No notification targets found");
        return Ok(());
    }

    state.emit_log(
        "INFO",
        "push",
        &format!("Sending daily notifications to {} users", targets.len()),
    );

    let api_key = &state.config.anthropic_api_key;

    for (user_id, device_token, profile) in &targets {
        let message = match generate_notification_message(api_key, profile.as_deref()).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!(
                    "Failed to generate notification for {}: {}",
                    &user_id[..8.min(user_id.len())],
                    e
                );
                continue;
            }
        };

        match apns::send_push(&apns_client, device_token, &bundle_id, "anky", &message).await {
            Ok(_) => {
                tracing::debug!("Push sent to user {}", &user_id[..8.min(user_id.len())]);
            }
            Err(e) => {
                tracing::error!(
                    "Push failed for user {}: {}",
                    &user_id[..8.min(user_id.len())],
                    e
                );
            }
        }
    }

    state.emit_log(
        "INFO",
        "push",
        &format!(
            "Daily push cycle complete — {} targets processed",
            targets.len()
        ),
    );

    Ok(())
}

async fn generate_notification_message(
    api_key: &str,
    profile: Option<&str>,
) -> anyhow::Result<String> {
    if api_key.is_empty() {
        // Fallback template when no API key
        return Ok("the page is waiting. 8 minutes. no backspace.".to_string());
    }

    let context = match profile {
        Some(p) if !p.is_empty() => format!(
            "This user's psychological profile from their past writing sessions:\n{}\n\n",
            p
        ),
        _ => String::new(),
    };

    let system =
        "You write extremely short push notification messages for anky, a writing practice app. \
        The user writes stream-of-consciousness for 8 minutes with no backspace or delete. \
        Your message should be 1-2 sentences max, under 100 characters ideally. \
        Be direct, slightly provocative, never cheesy or corporate. \
        Sound like a mirror, not a cheerleader. Never use exclamation marks. \
        Examples of good messages: \
        'you haven\\'t written in 3 days. the page doesn\\'t judge. 8 minutes.' \
        'what are you avoiding today. write it down.' \
        'your unconscious has something to say. will you listen.'";

    let user_msg = format!(
        "{}Generate a single push notification message to bring this user back to write. \
        Just output the message text, nothing else.",
        context
    );

    let result =
        claude::call_claude_public(api_key, "claude-haiku-4-5-20251001", system, &user_msg, 100)
            .await?;

    // Clean up: remove quotes if Claude wraps the output
    let text = result.text.trim().trim_matches('"').trim().to_string();
    Ok(text)
}
