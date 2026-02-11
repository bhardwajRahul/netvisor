use posthog_rs::{ClientOptions, Event};

pub struct PosthogService {
    client: posthog_rs::Client,
}

impl PosthogService {
    pub async fn new(api_key: String, api_host: String) -> Self {
        let options = ClientOptions::from((api_key.as_str(), api_host.as_str()));
        let client = posthog_rs::client(options).await;
        Self { client }
    }

    pub async fn capture(
        &self,
        event_name: &str,
        distinct_id: &str,
        properties: serde_json::Value,
    ) {
        let mut event = Event::new(event_name, distinct_id);
        if let Some(props) = properties.as_object() {
            for (key, value) in props {
                // insert_prop requires a Serialize type; serde_json::Value implements it
                if let Err(e) = event.insert_prop(key, value) {
                    tracing::warn!(key = %key, error = %e, "Failed to insert PostHog event property");
                }
            }
        }

        if let Err(e) = self.client.capture(event).await {
            tracing::warn!(event = %event_name, error = %e, "Failed to send event to PostHog");
        }
    }
}
