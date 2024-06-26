use {
    crate::{
        error::{Error, Error::InvalidMultipartBody},
        handlers::validate_tenant_request,
        increment_counter,
        state::AppState,
        stores::tenant::{TenantApnsUpdateAuth, TenantApnsUpdateParams},
    },
    a2::ClientConfig,
    axum::{
        extract::{Multipart, Path, State},
        http::HeaderMap,
        Json,
    },
    base64::Engine,
    serde::{Deserialize, Serialize},
    std::sync::Arc,
    tracing::{error, instrument, warn},
};

#[derive(Deserialize)]
pub struct ApnsUpdateBody {
    pub apns_topic: Option<String>,

    pub apns_certificate: Option<String>,
    pub apns_certificate_password: Option<String>,

    pub apns_pkcs8_pem: Option<String>,
    pub apns_key_id: Option<String>,
    pub apns_team_id: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ApnsSqlUpdate {
    pub topic: Option<String>,

    pub auth: Option<TenantApnsUpdateAuth>,
}

impl ApnsUpdateBody {
    pub fn validate(&self) -> Result<ApnsSqlUpdate, Error> {
        // Match cases when the input is not valid and return false.
        // Input is valid if certificate and certificate_password is included for
        // updates. topic is required for new tenants

        match (
            &self.apns_topic,
            &self.apns_certificate,
            &self.apns_certificate_password,
            &self.apns_pkcs8_pem,
            &self.apns_key_id,
            &self.apns_team_id,
        ) {
            // Update Topic
            (Some(topic), None, None, None, None, None) => Ok(ApnsSqlUpdate {
                topic: Some(topic.clone()),
                auth: None,
            }),
            // Update Certificate
            (Some(topic), Some(certificate), Some(password), None, None, None) => {
                Ok(ApnsSqlUpdate {
                    topic: Some(topic.clone()),
                    auth: Some(TenantApnsUpdateAuth::Certificate {
                        apns_certificate: certificate.clone(),
                        apns_certificate_password: password.clone(),
                    }),
                })
            }
            (Some(topic), Some(certificate), None, None, None, None) => Ok(ApnsSqlUpdate {
                topic: Some(topic.clone()),
                auth: Some(TenantApnsUpdateAuth::Certificate {
                    apns_certificate: certificate.clone(),
                    apns_certificate_password: String::new(),
                }),
            }),
            (None, Some(certificate), Some(password), None, None, None) => Ok(ApnsSqlUpdate {
                topic: None,
                auth: Some(TenantApnsUpdateAuth::Certificate {
                    apns_certificate: certificate.clone(),
                    apns_certificate_password: password.clone(),
                }),
            }),
            (None, Some(certificate), None, None, None, None) => Ok(ApnsSqlUpdate {
                topic: None,
                auth: Some(TenantApnsUpdateAuth::Certificate {
                    apns_certificate: certificate.clone(),
                    apns_certificate_password: String::new(),
                }),
            }),
            // Update Token
            (Some(topic), None, None, Some(pkcs8_pem), Some(key_id), Some(team_id)) => {
                Ok(ApnsSqlUpdate {
                    topic: Some(topic.clone()),
                    auth: Some(TenantApnsUpdateAuth::Token {
                        apns_pkcs8_pem: pkcs8_pem.clone(),
                        apns_key_id: key_id.clone(),
                        apns_team_id: team_id.clone(),
                    }),
                })
            }
            (None, None, None, Some(pkcs8_pem), Some(key_id), Some(team_id)) => Ok(ApnsSqlUpdate {
                topic: None,
                auth: Some(TenantApnsUpdateAuth::Token {
                    apns_pkcs8_pem: pkcs8_pem.clone(),
                    apns_key_id: key_id.clone(),
                    apns_team_id: team_id.clone(),
                }),
            }),
            // All other cases are invalid
            _ => Err(InvalidMultipartBody),
        }
    }
}

#[derive(Serialize)]
pub struct UpdateTenantApnsResponse {
    success: bool,
}

#[instrument(skip_all, name = "update_apns_handler")]
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    mut form_body: Multipart,
) -> Result<Json<UpdateTenantApnsResponse>, Error> {
    // JWT verification
    #[cfg(feature = "cloud")]
    let jwt_verification_result =
        validate_tenant_request(&state.jwt_validation_client, &headers, &id).await;

    #[cfg(not(feature = "cloud"))]
    let jwt_verification_result = validate_tenant_request(&state.jwt_validation_client, &headers);

    if let Err(e) = jwt_verification_result {
        error!(
            tenant_id = %id,
            err = ?e,
            "JWT verification failed"
        );
        return Err(e);
    }

    // Ensure tenant real
    let _existing_tenant = state.tenant_store.get_tenant(&id).await?;

    // ---- retrieve body from form
    let mut body = ApnsUpdateBody {
        apns_topic: None,

        apns_certificate: None,
        apns_certificate_password: None,

        apns_pkcs8_pem: None,
        apns_key_id: None,
        apns_team_id: None,
    };
    while let Some(field) = form_body.next_field().await? {
        let name = field.name().unwrap_or("unknown").to_string();

        // Check the lowercase name against list of known names for struct
        match name.to_lowercase().as_str() {
            "apns_topic" => {
                body.apns_topic = Some(field.text().await?);
            }
            "apns_certificate" => {
                let data = field.bytes().await?;
                let encoded_certificate = base64::engine::general_purpose::STANDARD.encode(&data);
                body.apns_certificate = Some(encoded_certificate);
            }
            "apns_certificate_password" => {
                body.apns_certificate_password = Some(field.text().await?);
            }
            "apns_pkcs8_pem" => {
                let data = field.bytes().await?;
                let encoded_p8_certificate =
                    base64::engine::general_purpose::STANDARD.encode(&data);
                body.apns_pkcs8_pem = Some(encoded_p8_certificate);
            }
            "apns_key_id" => {
                body.apns_key_id = Some(field.text().await?);
            }
            "apns_team_id" => {
                body.apns_team_id = Some(field.text().await?);
            }
            _ => {
                // Unknown field, ignored
            }
        };
    }

    let apns_updates = body.validate()?;

    if let Some(topic) = apns_updates.topic {
        // Just update topic
        let update_body = TenantApnsUpdateParams { apns_topic: topic };

        let _new_tenant = state
            .tenant_store
            .update_tenant_apns(&id, update_body)
            .await?;

        if apns_updates.auth.is_none() {
            // Breakout early as there are no auth updates

            increment_counter!(state.metrics, tenant_apns_updates);

            return Ok(Json(UpdateTenantApnsResponse { success: true }));
        }
    }

    // ---- Checks
    if let Some(auth_change) = apns_updates.auth.clone() {
        match auth_change {
            TenantApnsUpdateAuth::Certificate {
                apns_certificate,
                apns_certificate_password,
            } => {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(apns_certificate.into_bytes())?;
                match a2::Client::certificate(
                    &mut std::io::Cursor::new(decoded),
                    &apns_certificate_password,
                    ClientConfig::new(a2::Endpoint::Sandbox),
                ) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        warn!("Error validating APNS certificate on update: {:?}", e);
                        Err(Error::BadApnsCredentials)
                    }
                }
            }
            TenantApnsUpdateAuth::Token {
                apns_pkcs8_pem,
                apns_key_id,
                apns_team_id,
            } => {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(apns_pkcs8_pem.into_bytes())?;
                match a2::Client::token(
                    &mut std::io::Cursor::new(decoded),
                    apns_key_id,
                    apns_team_id,
                    ClientConfig::new(a2::Endpoint::Sandbox),
                ) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        warn!("Error validating APNS token on update: {:?}", e);
                        Err(Error::BadApnsCredentials)
                    }
                }
            }
        }?;
    }

    // ---- handler
    if let Some(auth) = apns_updates.auth {
        let new_tenant = state
            .tenant_store
            .update_tenant_apns_auth(&id, auth)
            .await?;

        increment_counter!(state.metrics, tenant_apns_updates);

        if new_tenant.suspended {
            // If suspended, it can be restored now because valid credentials have been
            // provided
            state.tenant_store.unsuspend_tenant(&new_tenant.id).await?;
        }

        return Ok(Json(UpdateTenantApnsResponse { success: true }));
    }

    // No auth updates or topic updates were carried out therefore the body was
    // invalid
    Err(InvalidMultipartBody)
}
