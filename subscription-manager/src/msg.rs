use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use secret_toolkit::permit::Permit;


// Struct for the message used to instantiate the contract
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

// Enum representing the different executable messages that the contract can handle
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Message to register a new subscriber using a public key
    RegisterSubscriber { public_key: String },

    // Message to remove a subscriber using a public key
    RemoveSubscriber { public_key: String },

    // Message to set a new admin for the contract using a public key
    SetAdmin { public_key: String },
    // Message to add an API key
    AddApiKey { api_key: String },
    // Message to revoke an API key
    RevokeApiKey { api_key: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {
    Migrate {},
    StdError {},
}

// Enum representing the different query messages that the contract can respond to
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    SubscriberStatusWithPermit {
        public_key: String,
        permit: Permit,
    },
    GetAdmin {},
    ApiKeysWithPermit {
        permit: Permit,
    },
}

// Struct used to respond to a query about a subscriber's status
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct SubscriberStatusResponse {
    // Indicates if the subscriber is active or not
    pub active: bool,
}

// Structure for API keys to respond to a query
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct ApiKeyResponse {
    // Previously `key: String`,
    // Maybe rename to `hash: String` or `hashed_key: String`.
    pub hashed_key: String,
}

// Structure for GetApiKeysResponse
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct GetApiKeysResponse {
    pub api_keys: Vec<ApiKeyResponse>,
}