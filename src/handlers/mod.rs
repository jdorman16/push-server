use {
    crate::{
        error::{Error::InvalidAuthentication, Result},
        jwt_validation::{Claims, JwtValidationClient},
    },
    axum::{
        http::{header::AUTHORIZATION, HeaderMap},
        response::IntoResponse,
        Json,
    },
    hyper::StatusCode,
    jsonwebtoken::TokenData,
    relay_rpc::{
        domain::ClientId,
        jwt::{JwtBasicClaims, VerifyableClaims},
    },
    serde_json::{json, Value},
    std::{collections::HashSet, string::ToString},
    tracing::{debug, instrument},
};

// Push
pub mod delete_client;
pub mod metrics;
pub mod push_message;
pub mod register_client;
#[cfg(not(feature = "multitenant"))]
pub mod single_tenant_wrappers;
// Tenant Management
#[cfg(feature = "multitenant")]
pub mod create_tenant;
#[cfg(feature = "multitenant")]
pub mod delete_apns;
#[cfg(feature = "multitenant")]
pub mod delete_fcm;
#[cfg(feature = "multitenant")]
pub mod delete_fcm_v1;
#[cfg(feature = "multitenant")]
pub mod delete_tenant;
#[cfg(feature = "multitenant")]
pub mod get_tenant;
pub mod health;
pub mod rate_limit_test;
#[cfg(feature = "multitenant")]
pub mod update_apns;
#[cfg(feature = "multitenant")]
pub mod update_fcm;
#[cfg(feature = "multitenant")]
pub mod update_fcm_v1;

pub const DECENTRALIZED_IDENTIFIER_PREFIX: &str = "did:key:";

#[instrument(skip_all)]
pub fn authenticate_client<F>(headers: HeaderMap, aud: &str, check: F) -> Result<bool>
where
    F: FnOnce(Option<ClientId>) -> bool,
{
    return if let Some(auth_header) = headers.get(axum::http::header::AUTHORIZATION) {
        let header_str = auth_header.to_str()?;

        let claims = JwtBasicClaims::try_from_str(header_str).map_err(|e| {
            debug!("Invalid claims: {:?}", e);
            e
        })?;
        claims
            .verify_basic(&HashSet::from([aud.to_string()]), None)
            .map_err(|e| {
                debug!("Failed to verify_basic: {:?}", e);
                e
            })?;
        let client_id: ClientId = claims.iss.into();
        Ok(check(Some(client_id)))
    } else {
        // Note: Authentication is not required right now to ensure that this is a
        // non-breaking change, eventually it will be required and this should default
        // to returning `Err(MissingAuthentication)` or `Err(InvalidAuthentication)`
        Ok(true)
    };
}

#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorLocation {
    Body,
    // Note (Harry): Spec supports this but it currently isn't used
    // Query,
    Header,
    Path,
    Unknown,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ResponseStatus {
    Success,
    Failure,
}

#[derive(serde::Serialize)]
pub struct ErrorField {
    pub field: String,
    pub description: String,
    pub location: ErrorLocation,
}

#[derive(serde::Serialize)]
pub struct ResponseError {
    pub name: String,
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct Response {
    pub status: ResponseStatus,
    #[serde(skip_serializing)]
    pub status_code: StatusCode,
    pub errors: Option<Vec<ResponseError>>,
    pub fields: Option<Vec<ErrorField>>,
}

impl Response {
    pub fn new_success(status: StatusCode) -> Self {
        Response {
            status: ResponseStatus::Success,
            status_code: status,
            errors: None,
            fields: None,
        }
    }

    pub fn new_failure(
        status: StatusCode,
        errors: Vec<ResponseError>,
        fields: Vec<ErrorField>,
    ) -> Self {
        Response {
            status: ResponseStatus::Failure,
            status_code: status,
            errors: Some(errors),
            fields: Some(fields),
        }
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code;
        let json: Json<Value> = self.into();

        (status, json).into_response()
    }
}

impl From<Response> for Json<Value> {
    fn from(value: Response) -> Self {
        Json(json!(value))
    }
}

impl Default for Response {
    fn default() -> Self {
        Response::new_success(StatusCode::OK)
    }
}

fn validate_jwt(
    jwt_validation_client: &JwtValidationClient,
    headers: &HeaderMap,
) -> Result<TokenData<Claims>> {
    if let Some(token_data) = headers.get(AUTHORIZATION) {
        // TODO Clients should always use `Bearer`, migrate them (if not already) and remove this optionality
        // TODO Specific not-bearer token error
        let jwt = token_data.to_str()?.to_string().replace("Bearer ", "");
        jwt_validation_client
            .is_valid_token(jwt)
            .map_err(|_| InvalidAuthentication)
    } else {
        // TODO specific missing Authorization header error
        Err(InvalidAuthentication)
    }
}

#[cfg(feature = "cloud")]
#[instrument(skip_all, fields(project_id = %project_id))]
pub async fn validate_tenant_request(
    jwt_validation_client: &JwtValidationClient,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<()> {
    let token_data = validate_jwt(jwt_validation_client, headers)?;
    if token_data.claims.sub == project_id {
        Ok(())
    } else {
        // TODO specific wrong `sub` error
        Err(InvalidAuthentication)
    }
}

#[cfg(not(feature = "cloud"))]
#[instrument(skip_all)]
pub fn validate_tenant_request(
    jwt_validation_client: &JwtValidationClient,
    headers: &HeaderMap,
) -> Result<()> {
    validate_jwt(jwt_validation_client, headers).map(|_| ())
}
