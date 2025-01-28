use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use secret_toolkit::storage::{Keymap};

use cosmwasm_std::{Addr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

// Key for accessing the configuration state in storage
pub static CONFIG_KEY: &[u8] = b"config";

// Keymap for storing subscribers' information, using public keys as keys
pub static SB_MAP: Keymap<String, Subscriber> = Keymap::new(b"SB_MAP");

// Keymap for storing API keys
pub static API_KEY_MAP: Keymap<String, ApiKey> = Keymap::new(b"API_KEY_MAP");

// Structure representing the state of the contract
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct State {
    // Address of the admin
    pub admin: Addr,
}

// Structure representing a subscriber's information
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Subscriber {
    // Status of the subscriber (active or not)
    pub status: bool,
}

// Structure representing an API key to be stored
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct ApiKey {}

// Function to access and modify the configuration state
pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

// Function to read the configuration state without modifying it
pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}
