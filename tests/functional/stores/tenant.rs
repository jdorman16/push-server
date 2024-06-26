use {
    crate::context::StoreContext,
    echo_server::stores::tenant::{
        TenantApnsUpdateAuth, TenantApnsUpdateParams, TenantFcmUpdateParams,
        TenantFcmV1UpdateParams, TenantUpdateParams,
    },
    test_context::test_context,
    uuid::Uuid,
};

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_creation(ctx: &mut StoreContext) {
    let res = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await;

    assert!(res.is_ok())
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_deletion(ctx: &mut StoreContext) {
    let id = Uuid::new_v4().to_string();

    let res = ctx
        .tenants
        .create_tenant(TenantUpdateParams { id: id.clone() })
        .await;

    assert!(res.is_ok());

    let delete_res = ctx.tenants.delete_tenant(&id).await;

    assert!(delete_res.is_ok())
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_get(ctx: &mut StoreContext) {
    let id = Uuid::new_v4().to_string();

    let res = ctx
        .tenants
        .create_tenant(TenantUpdateParams { id: id.clone() })
        .await;

    assert!(res.is_ok());

    let tenant_res = ctx.tenants.get_tenant(&id).await;

    assert!(tenant_res.is_ok());

    let tenant = tenant_res.expect("failed to unwrap tenant");

    assert_eq!(tenant.id, id);
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_fcm(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_fcm(
            &tenant.id,
            TenantFcmUpdateParams {
                fcm_api_key: "test-api-key".to_string(),
            },
        )
        .await;

    assert!(res.is_ok())
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_delete_fcm(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_fcm(
            &tenant.id,
            TenantFcmUpdateParams {
                fcm_api_key: "test-api-key".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(res.fcm_api_key, Some("test-api-key".to_owned()));

    let res = ctx
        .tenants
        .update_tenant_fcm_v1(
            &tenant.id,
            TenantFcmV1UpdateParams {
                fcm_v1_credentials: "test-credentials".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(res.fcm_v1_credentials, Some("test-credentials".to_owned()));

    let res = ctx
        .tenants
        .update_tenant_delete_fcm(&tenant.id)
        .await
        .unwrap();
    assert_eq!(res.fcm_api_key, None);

    let res = ctx.tenants.get_tenant(&tenant.id).await.unwrap();
    assert_eq!(res.fcm_api_key, None);
    assert_eq!(res.fcm_v1_credentials, Some("test-credentials".to_owned()));
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_fcm_v1(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_fcm_v1(
            &tenant.id,
            TenantFcmV1UpdateParams {
                fcm_v1_credentials: "test-credentials".to_string(),
            },
        )
        .await;

    assert!(res.is_ok())
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_delete_fcm_v1(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_fcm_v1(
            &tenant.id,
            TenantFcmV1UpdateParams {
                fcm_v1_credentials: "test-credentials".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(res.fcm_v1_credentials, Some("test-credentials".to_owned()));

    let res = ctx
        .tenants
        .update_tenant_fcm(
            &tenant.id,
            TenantFcmUpdateParams {
                fcm_api_key: "test-api-key".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(res.fcm_api_key, Some("test-api-key".to_owned()));

    let res = ctx
        .tenants
        .update_tenant_delete_fcm_v1(&tenant.id)
        .await
        .unwrap();
    assert_eq!(res.fcm_v1_credentials, None);

    let res = ctx.tenants.get_tenant(&tenant.id).await.unwrap();
    assert_eq!(res.fcm_v1_credentials, None);
    assert_eq!(res.fcm_api_key, Some("test-api-key".to_owned()));
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_apns(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_apns(
            &tenant.id,
            TenantApnsUpdateParams {
                apns_topic: "com.walletconect.exampleapp".to_string(),
            },
        )
        .await;

    assert!(res.is_ok())
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_apns_certificate_auth(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_apns_auth(
            &tenant.id,
            TenantApnsUpdateAuth::Certificate {
                apns_certificate: "example-certificate-string".to_string(),
                apns_certificate_password: "password123".to_string(),
            },
        )
        .await;

    assert!(res.is_ok())
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_apns_token_auth(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_apns_auth(
            &tenant.id,
            TenantApnsUpdateAuth::Token {
                apns_pkcs8_pem: "example-pem-string".to_string(),
                apns_key_id: "123".to_string(),
                apns_team_id: "456".to_string(),
            },
        )
        .await;

    assert!(res.is_ok())
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_delete_apns_certificate_auth(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_apns_auth(
            &tenant.id,
            TenantApnsUpdateAuth::Certificate {
                apns_certificate: "example-certificate-string".to_string(),
                apns_certificate_password: "password123".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        res.apns_certificate,
        Some("example-certificate-string".to_owned())
    );
    assert_eq!(
        res.apns_certificate_password,
        Some("password123".to_owned())
    );

    let res = ctx
        .tenants
        .update_tenant_apns(
            &tenant.id,
            TenantApnsUpdateParams {
                apns_topic: "com.walletconect.exampleapp".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        res.apns_topic,
        Some("com.walletconect.exampleapp".to_owned())
    );
    assert_eq!(
        res.apns_certificate,
        Some("example-certificate-string".to_owned())
    );
    assert_eq!(
        res.apns_certificate_password,
        Some("password123".to_owned())
    );

    let res = ctx
        .tenants
        .update_tenant_delete_apns(&tenant.id)
        .await
        .unwrap();
    assert_eq!(res.apns_topic, None);
    assert_eq!(res.apns_pkcs8_pem, None);
    assert_eq!(res.apns_key_id, None);
    assert_eq!(res.apns_team_id, None);
    assert_eq!(res.apns_certificate, None);
    assert_eq!(res.apns_certificate_password, None);
}

#[test_context(StoreContext)]
#[tokio::test]
async fn tenant_delete_apns_token_auth(ctx: &mut StoreContext) {
    let tenant = ctx
        .tenants
        .create_tenant(TenantUpdateParams {
            id: Uuid::new_v4().to_string(),
        })
        .await
        .expect("creation failed");

    let res = ctx
        .tenants
        .update_tenant_apns(
            &tenant.id,
            TenantApnsUpdateParams {
                apns_topic: "com.walletconect.exampleapp".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        res.apns_topic,
        Some("com.walletconect.exampleapp".to_owned())
    );

    let res = ctx
        .tenants
        .update_tenant_apns_auth(
            &tenant.id,
            TenantApnsUpdateAuth::Token {
                apns_pkcs8_pem: "example-pem-string".to_string(),
                apns_key_id: "123".to_string(),
                apns_team_id: "456".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(res.apns_pkcs8_pem, Some("example-pem-string".to_owned()));
    assert_eq!(res.apns_key_id, Some("123".to_owned()));
    assert_eq!(res.apns_team_id, Some("456".to_owned()));
    assert_eq!(
        res.apns_topic,
        Some("com.walletconect.exampleapp".to_owned())
    );

    let res = ctx
        .tenants
        .update_tenant_delete_apns(&tenant.id)
        .await
        .unwrap();
    assert_eq!(res.apns_topic, None);
    assert_eq!(res.apns_pkcs8_pem, None);
    assert_eq!(res.apns_key_id, None);
    assert_eq!(res.apns_team_id, None);
    assert_eq!(res.apns_certificate, None);
    assert_eq!(res.apns_certificate_password, None);
}
