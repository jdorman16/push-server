use {
    std::time::Instant,
    wc::metrics::{
        otel::{
            metrics::{Counter, Histogram},
            KeyValue,
        },
        ServiceMetrics,
    },
};

#[derive(Clone)]
pub struct Metrics {
    pub received_notifications: Counter<u64>,
    pub sent_fcm_notifications: Counter<u64>,
    pub sent_fcm_v1_notifications: Counter<u64>,
    pub sent_apns_notifications: Counter<u64>,

    pub registered_clients: Counter<u64>,
    pub registered_tenants: Counter<u64>,

    pub tenant_apns_updates: Counter<u64>,
    pub tenant_fcm_updates: Counter<u64>,
    pub tenant_fcm_v1_updates: Counter<u64>,

    pub tenant_suspensions: Counter<u64>,
    pub client_suspensions: Counter<u64>,

    postgres_queries: Counter<u64>,
    postgres_query_latency: Histogram<u64>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        ServiceMetrics::init_with_name("echo-server");
        let meter = ServiceMetrics::meter();

        let clients_counter = meter
            .u64_counter("registered_clients")
            .with_description("The number of currently registered clients")
            .init();

        let tenants_counter = meter
            .u64_counter("registered_tenants")
            .with_description("The number of currently registered tenants")
            .init();

        let received_notification_counter = meter
            .u64_counter("received_notifications")
            .with_description("The number of notification received")
            .init();

        let sent_fcm_notification_counter = meter
            .u64_counter("sent_fcm_notifications")
            .with_description("The number of notifications sent to FCM")
            .init();

        let sent_fcm_v1_notification_counter = meter
            .u64_counter("sent_fcm_v1_notifications")
            .with_description("The number of notifications sent to FCM")
            .init();

        let sent_apns_notification_counter = meter
            .u64_counter("sent_apns_notifications")
            .with_description("The number of notifications sent to APNS")
            .init();

        let tenant_apns_updates_counter = meter
            .u64_counter("tenant_apns_updates")
            .with_description("The number of times tenants have updated their APNS")
            .init();

        let tenant_fcm_updates_counter = meter
            .u64_counter("tenant_fcm_updates")
            .with_description("The number of times tenants have updated their FCM")
            .init();

        let tenant_fcm_v1_updates_counter = meter
            .u64_counter("tenant_fcm_v1_updates")
            .with_description("The number of times tenants have updated their FCM")
            .init();

        let tenant_suspensions_counter = meter
            .u64_counter("tenant_suspensions")
            .with_description("The number of tenants that have been suspended")
            .init();

        let client_suspensions_counter = meter
            .u64_counter("client_suspensions")
            .with_description("The number of clients that have been suspended")
            .init();

        let postgres_queries: Counter<u64> = meter
            .u64_counter("postgres_queries")
            .with_description("The number of Postgres queries executed")
            .init();

        let postgres_query_latency: Histogram<u64> = meter
            .u64_histogram("postgres_query_latency")
            .with_description("The latency Postgres queries")
            .init();

        Metrics {
            registered_clients: clients_counter,
            received_notifications: received_notification_counter,
            sent_fcm_notifications: sent_fcm_notification_counter,
            sent_fcm_v1_notifications: sent_fcm_v1_notification_counter,
            sent_apns_notifications: sent_apns_notification_counter,
            registered_tenants: tenants_counter,
            tenant_apns_updates: tenant_apns_updates_counter,
            tenant_fcm_updates: tenant_fcm_updates_counter,
            tenant_fcm_v1_updates: tenant_fcm_v1_updates_counter,
            tenant_suspensions: tenant_suspensions_counter,
            client_suspensions: client_suspensions_counter,
            postgres_queries,
            postgres_query_latency,
        }
    }

    pub fn postgres_query(&self, query_name: &'static str, start: Instant) {
        let elapsed = start.elapsed();

        let attributes = [KeyValue::new("name", query_name)];
        self.postgres_queries.add(1, &attributes);
        self.postgres_query_latency
            .record(elapsed.as_millis() as u64, &attributes);
    }
}
