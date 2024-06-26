use {
    super::{LegacyPushMessage, PushMessage},
    crate::{blob::DecryptedPayloadBlob, error::Error, providers::PushProvider},
    async_trait::async_trait,
    fcm::{ErrorReason, FcmError, FcmResponse, MessageBuilder, NotificationBuilder, Priority},
    std::fmt::{Debug, Formatter},
    tracing::{debug, instrument},
};

pub struct FcmProvider {
    api_key: String,
    client: fcm::Client,
}

impl FcmProvider {
    pub fn new(api_key: String) -> Self {
        FcmProvider {
            api_key,
            client: fcm::Client::new(),
        }
    }
}

#[async_trait]
impl PushProvider for FcmProvider {
    #[instrument(name = "send_fcm_notification")]
    async fn send_notification(
        &self,
        token: String,
        body: PushMessage,
    ) -> crate::error::Result<()> {
        let mut message_builder = MessageBuilder::new(self.api_key.as_str(), token.as_str());

        let result = match body {
            PushMessage::RawPushMessage(message) => {
                // Sending `always_raw` encrypted message
                debug!("Sending raw encrypted message");
                message_builder
                    .data(&message)
                    .map_err(Error::InternalSerializationError)?;
                set_message_priority_high(&mut message_builder);
                let fcm_message = message_builder.finalize();
                self.client.send(fcm_message).await
            }
            PushMessage::LegacyPushMessage(LegacyPushMessage { id: _, payload }) => {
                if payload.is_encrypted() {
                    debug!("Sending legacy `is_encrypted` message");
                    message_builder
                        .data(&payload)
                        .map_err(Error::InternalSerializationError)?;
                    set_message_priority_high(&mut message_builder);
                    let fcm_message = message_builder.finalize();
                    self.client.send(fcm_message).await
                } else {
                    debug!("Sending plain message");
                    let blob = DecryptedPayloadBlob::from_base64_encoded(&payload.blob)?;

                    let mut notification_builder = NotificationBuilder::new();
                    notification_builder.title(blob.title.as_str());
                    notification_builder.body(blob.body.as_str());
                    let notification = notification_builder.finalize();

                    message_builder.notification(notification);
                    message_builder
                        .data(&payload.to_owned())
                        .map_err(Error::InternalSerializationError)?;
                    let fcm_message = message_builder.finalize();
                    self.client.send(fcm_message).await
                }
            }
        };

        match result {
            Ok(val) => {
                let FcmResponse { error, .. } = val;
                if let Some(error) = error {
                    match error {
                        ErrorReason::MissingRegistration => Err(Error::BadDeviceToken(
                            "Missing registration for token".into(),
                        )),
                        ErrorReason::InvalidRegistration => {
                            Err(Error::BadDeviceToken("Invalid token registration".into()))
                        }
                        ErrorReason::NotRegistered => {
                            Err(Error::BadDeviceToken("Token is not registered".into()))
                        }
                        ErrorReason::InvalidApnsCredential => Err(Error::BadApnsCredentials),
                        e => Err(Error::FcmResponse(e)),
                    }
                } else {
                    Ok(())
                }
            }
            Err(e) => match e {
                FcmError::Unauthorized => Err(Error::BadFcmApiKey),
                e => Err(Error::Fcm(e)),
            },
        }
    }
}

// Manual Impl Because `fcm::Client` does not derive anything and doesn't need
// to be accounted for

impl Clone for FcmProvider {
    fn clone(&self) -> Self {
        FcmProvider {
            api_key: self.api_key.clone(),
            client: fcm::Client::new(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.api_key.clone_from(&source.api_key);
        self.client = fcm::Client::new();
    }
}

impl PartialEq for FcmProvider {
    fn eq(&self, other: &Self) -> bool {
        self.api_key == other.api_key
    }
}

impl Debug for FcmProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[FcmProvider] api_key = {}", self.api_key)
    }
}

/// Setting message priority to high and content-available to true
/// on data-only messages or they don't show unless app is active
/// https://rnfirebase.io/messaging/usage#data-only-messages
fn set_message_priority_high(builder: &mut MessageBuilder) {
    builder.priority(Priority::High);
    builder.content_available(true);
}
