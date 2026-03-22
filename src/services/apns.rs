use crate::config::Config;
use a2::client::ClientConfig;
use a2::request::notification::{
    DefaultNotificationBuilder, NotificationBuilder, NotificationOptions,
};
use a2::{Client, Endpoint};
use anyhow::{Context, Result};

/// Build an APNs client from config. Returns None if credentials are not set.
pub fn build_client(config: &Config) -> Option<Client> {
    if config.apns_key_path.is_empty()
        || config.apns_key_id.is_empty()
        || config.apns_team_id.is_empty()
    {
        tracing::warn!("APNs credentials not configured — push notifications disabled");
        return None;
    }

    let key_file = match std::fs::File::open(&config.apns_key_path) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(
                "Failed to open APNs key file {}: {}",
                config.apns_key_path,
                e
            );
            return None;
        }
    };

    let mut reader = std::io::BufReader::new(key_file);

    let endpoint = if config.apns_environment == "sandbox" {
        Endpoint::Sandbox
    } else {
        Endpoint::Production
    };

    let client_config = ClientConfig::new(endpoint);

    match Client::token(
        &mut reader,
        &config.apns_key_id,
        &config.apns_team_id,
        client_config,
    ) {
        Ok(client) => Some(client),
        Err(e) => {
            tracing::error!("Failed to create APNs client: {}", e);
            None
        }
    }
}

/// Send a push notification to a single device.
pub async fn send_push(
    client: &Client,
    device_token: &str,
    bundle_id: &str,
    title: &str,
    body: &str,
) -> Result<()> {
    let options = NotificationOptions {
        apns_topic: Some(bundle_id),
        ..Default::default()
    };

    let payload = DefaultNotificationBuilder::new()
        .set_title(title)
        .set_body(body)
        .set_sound("default")
        .set_badge(1)
        .build(device_token, options);

    client.send(payload).await.context("APNs send failed")?;

    Ok(())
}
