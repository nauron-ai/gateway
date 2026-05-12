use serde::{Deserialize, Serialize};
use url::Url;
use utoipa::ToSchema;

use crate::error::GatewayError;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CallbackTarget {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

pub fn validate_callback_target(callback: Option<&CallbackTarget>) -> Result<(), GatewayError> {
    let Some(callback) = callback else {
        return Ok(());
    };

    let parsed = Url::parse(&callback.url).map_err(|_| GatewayError::InvalidField {
        field: "callback.url".to_string(),
        message: "callback.url must be a valid absolute URL".to_string(),
    })?;

    match parsed.scheme() {
        "http" | "https" => Ok(()),
        _ => Err(GatewayError::InvalidField {
            field: "callback.url".to_string(),
            message: "callback.url must use http or https scheme".to_string(),
        }),
    }
}
