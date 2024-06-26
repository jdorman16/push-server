use {
    crate::{
        error::Error::{
            FromRequestError, MissingAllSignatureHeader, MissingSignatureHeader,
            MissingTimestampHeader, ToBytesError,
        },
        state::State,
    },
    async_trait::async_trait,
    axum::{
        body::to_bytes,
        extract::{FromRequest, Request},
    },
    ed25519_dalek::{Signature, VerifyingKey},
    tracing::instrument,
};

pub const SIGNATURE_HEADER_NAME: &str = "X-Ed25519-Signature";
pub const TIMESTAMP_HEADER_NAME: &str = "X-Ed25519-Timestamp";

pub struct RequireValidSignature<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for RequireValidSignature<T>
where
    S: Send + Sync + State,
    T: FromRequest<S>,
{
    type Rejection = crate::error::Error;

    #[instrument(skip_all)]
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !state.validate_signatures() {
            // Skip signature validation
            return T::from_request(req, state)
                .await
                .map(Self)
                .map_err(|_| FromRequestError);
        }

        let state_binding = state.relay_client();
        let public_key = state_binding.get_verifying_key();

        let (parts, body_raw) = req.into_parts();
        const MAX_BODY: usize = 1024 * 1024 * 100; // prolly too big but better than usize::MAX
        let bytes = to_bytes(body_raw, MAX_BODY)
            .await
            .map_err(|_| ToBytesError)?;
        let body = String::from_utf8_lossy(&bytes);

        let signature_header = parts
            .headers
            .get(SIGNATURE_HEADER_NAME)
            .and_then(|header| header.to_str().ok());

        let timestamp_header = parts
            .headers
            .get(TIMESTAMP_HEADER_NAME)
            .and_then(|header| header.to_str().ok());

        match (signature_header, timestamp_header) {
            (Some(signature), Some(timestamp))
                if signature_is_valid(signature, timestamp, &body, public_key).await? =>
            {
                let req = Request::from_parts(parts, bytes.into());
                Ok(T::from_request(req, state)
                    .await
                    .map(Self)
                    .map_err(|_| FromRequestError)?)
            }
            (Some(_), None) => Err(MissingTimestampHeader),
            (None, Some(_)) => Err(MissingSignatureHeader),
            (None, None) => Err(MissingAllSignatureHeader),
            _ => Err(MissingAllSignatureHeader),
        }
    }
}

pub async fn signature_is_valid(
    signature: &str,
    timestamp: &str,
    body: &str,
    public_key: &VerifyingKey,
) -> Result<bool, crate::error::Error> {
    let sig_body = format!("{}.{}.{}", timestamp, body.len(), body);

    let sig_bytes = hex::decode(signature).map_err(crate::error::Error::Hex)?;
    let sig = Signature::try_from(sig_bytes.as_slice())?;

    Ok(public_key.verify_strict(sig_body.as_bytes(), &sig).is_ok())
}
