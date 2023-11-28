use {
    crate::state::{AppState, State},
    axum::{extract::State as ExtractState, http::StatusCode, response::IntoResponse, Json},
    serde::Serialize,
    std::sync::Arc,
};

#[derive(Serialize)]
struct HealthResponseFlags {
    pub multitenant: bool,
    pub metrics: bool,
}

#[derive(Serialize)]
struct HealthResponse {
    pub status: String,
    pub version: String,
    pub flags: HealthResponseFlags,
    pub features_enabled: Vec<String>,
}

pub async fn handler(ExtractState(state): ExtractState<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "OK".to_string(),
            version: state.build_info.crate_info.version.to_string(),
            flags: HealthResponseFlags {
                multitenant: state.is_multitenant(),
                metrics: state.metrics.is_some(),
            },
            features_enabled: state.build_info.crate_info.enabled_features.clone(),
        }),
    )
}
