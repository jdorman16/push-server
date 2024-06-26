#[cfg(feature = "analytics")]
use {crate::analytics::client_info::ClientInfo, axum_client_ip::SecureClientIp};
use {
    crate::{
        error::{
            Error::{EmptyField, InvalidAuthentication, ProviderNotAvailable},
            Result,
        },
        handlers::{authenticate_client, Response, DECENTRALIZED_IDENTIFIER_PREFIX},
        increment_counter,
        log::prelude::*,
        state::AppState,
        stores::client::Client,
    },
    axum::{
        extract::{Json, Path, State as StateExtractor},
        http::HeaderMap,
    },
    relay_rpc::domain::ClientId,
    serde::{Deserialize, Serialize},
    std::sync::Arc,
    tracing::instrument,
};

#[derive(Serialize, Deserialize)]
pub struct RegisterBody {
    pub client_id: ClientId,
    #[serde(rename = "type")]
    pub push_type: String,
    pub token: String,
    pub always_raw: Option<bool>,
}

#[instrument(skip_all, name = "register_client_handler")]
pub async fn handler(
    #[cfg(feature = "analytics")] SecureClientIp(client_ip): SecureClientIp,
    Path(tenant_id): Path<String>,
    StateExtractor(state): StateExtractor<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RegisterBody>,
) -> Result<Response> {
    if !authenticate_client(headers, &state.config.public_url, |client_id| {
        if let Some(client_id) = client_id {
            debug!(
                %tenant_id,
                requested_client_id = %body.client_id,
                token_client_id = %client_id,
                "client_id authentication checking"
            );
            client_id == body.client_id
        } else {
            debug!(
                %tenant_id,
                requested_client_id = %body.client_id,
                token_client_id = "unknown",
                "client_id verification failed: missing client_id"
            );
            false
        }
    })? {
        debug!(
            %tenant_id,
            requested_client_id = %body.client_id,
            token_client_id = "unknown",
            "client_id verification failed: invalid client_id"
        );
        return Err(InvalidAuthentication);
    }

    let push_type = body.push_type.as_str().try_into()?;
    let tenant = state.tenant_store.get_tenant(&tenant_id).await?;
    let supported_providers = tenant.providers();
    if !supported_providers.contains(&push_type) {
        return Err(ProviderNotAvailable(push_type.into()));
    }

    if body.token.is_empty() {
        return Err(EmptyField("token".to_string()));
    }

    let client_id = body
        .client_id
        .as_ref()
        .trim_start_matches(DECENTRALIZED_IDENTIFIER_PREFIX)
        .to_owned();

    let always_raw = body.always_raw.unwrap_or(false);
    state
        .client_store
        .create_client(
            &tenant_id,
            &client_id,
            Client {
                tenant_id: tenant_id.clone(),
                push_type,
                token: body.token,
                always_raw,
            },
            state.metrics.as_ref(),
        )
        .await?;

    debug!(
        %tenant_id, %client_id, %push_type, "registered client"
    );

    increment_counter!(state.metrics, registered_clients);

    // Analytics
    #[cfg(feature = "analytics")]
    tokio::spawn(async move {
        if let Some(analytics) = &state.analytics {
            let (country, continent, region) = analytics
                .lookup_geo_data(client_ip)
                .map_or((None, None, None), |geo| {
                    (geo.country, geo.continent, geo.region)
                });

            debug!(
                %tenant_id,
                %client_id,
                ip = %client_ip,
                "loaded geo data"
            );

            let msg = ClientInfo {
                region: region.map(|r| Arc::from(r.join(", "))),
                country,
                continent,
                project_id: tenant_id.into(),
                client_id: client_id.into(),
                push_provider: body.push_type.as_str().into(),
                always_raw,
                registered_at: wc::analytics::time::now(),
            };

            analytics.client(msg);
        }
    });

    Ok(Response::default())
}
