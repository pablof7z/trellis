use std::collections::BTreeSet;

use super::types::{ControlResource, DesiredAppConfig};

/// Builds a sorted string set from literal values.
pub fn ids<const N: usize>(values: [&str; N]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// Initial app config used by the script.
pub fn initial_config() -> DesiredAppConfig {
    DesiredAppConfig {
        app_id: "checkout".to_owned(),
        image: "checkout-api".to_owned(),
        version: "v1".to_owned(),
        replicas: 2,
        port: 8080,
        volumes: ids(["cache"]),
        secrets: ids(["payments"]),
    }
}

/// Updated app config used by the script.
pub fn updated_config() -> DesiredAppConfig {
    DesiredAppConfig {
        app_id: "checkout".to_owned(),
        image: "checkout-api".to_owned(),
        version: "v2".to_owned(),
        replicas: 3,
        port: 9090,
        volumes: ids(["cache"]),
        secrets: ids(["payments"]),
    }
}

/// Builds a worker resource for the sample app.
pub fn worker_resource(version: &str, ordinal: u32) -> ControlResource {
    ControlResource::Worker {
        app_id: "checkout".to_owned(),
        ordinal,
        image: "checkout-api".to_owned(),
        version: version.to_owned(),
    }
}

/// Builds the initial port resource.
pub fn initial_port_resource() -> ControlResource {
    ControlResource::Port {
        app_id: "checkout".to_owned(),
        port: 8080,
    }
}

/// Builds the updated port resource.
pub fn updated_port_resource() -> ControlResource {
    ControlResource::Port {
        app_id: "checkout".to_owned(),
        port: 9090,
    }
}
