use serde::Deserialize;

/// Configuration for the DiscordReviewSink
/// The fields are derived from a URL like
/// https://discord.com/api/webhooks/1234567891234567890/aslkdjsfjhlksdjfhkjsdhflkjsdhflksdahflkjadshflkjsdhflkjdsasdskfjsdlk
#[derive(Debug, Deserialize, Clone)]
pub struct DiscordReviewSinkConfig {
    /// The ID of a webhook. This is the first part of the webhook URL
    pub webhook_id: u64,

    /// The token of a webhook. This is the second part of the webhook URL
    pub webhook_token: String,
}
