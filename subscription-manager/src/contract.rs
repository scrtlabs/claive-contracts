use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult
};
use secret_toolkit::permit::{validate, Permit};
use sha2::{Digest, Sha256};
use crate::msg::{ApiKeyResponse, ExecuteMsg, GetApiKeysResponse, InstantiateMsg, MigrateMsg, QueryMsg, SubscriberStatusResponse};
use crate::state::{config, config_read, ApiKey, State, Subscriber, API_KEY_MAP, SB_MAP};

// Entry point for contract initialization
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    // Set the admin to the sender who initializes the contract
    let state = State {
        admin: info.sender.clone(),
    };

    // Log a debug message
    deps.api
        .debug(format!("Contract was initialized by {}", info.sender).as_str());

    // Save the initial state
    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

// Entry point for executing messages
#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        // Handle registration of a subscriber
        ExecuteMsg::RegisterSubscriber { public_key } => try_register_subscriber(deps, info, public_key),
        // Handle removal of a subscriber
        ExecuteMsg::RemoveSubscriber { public_key } => try_remove_subscriber(deps, info, public_key),
        // Handle setting a new admin
        ExecuteMsg::SetAdmin { public_key } => try_set_admin(deps, info, public_key),
        // Handle adding an API key
        ExecuteMsg::AddApiKey { api_key } => try_add_api_key(deps, info, api_key),
        // Handle revoking an API key
        ExecuteMsg::RevokeApiKey { api_key } => try_revoke_api_key(deps, info, api_key),
    }
}

pub fn try_add_api_key(
    deps: DepsMut,
    info: MessageInfo,
    api_key: String,
) -> StdResult<Response> {
    let config = config_read(deps.storage);
    let state = config.load()?;

    // Check if the sender is the admin
    if info.sender != state.admin {
        return Err(StdError::generic_err("Only admin can add API keys"));
    }

    // 1. Compute the hash of the provided api_key
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());
    // This is a hex-encoded string of 64 hex characters.

    // 2. Check if this hash already exists
    if API_KEY_MAP.contains(deps.storage, &key_hash) {
        return Err(StdError::generic_err("API key (hash) already exists"));
    }

    // 3. Insert the hash into the map
    let api_key_data = ApiKey {};
    API_KEY_MAP
        .insert(deps.storage, &key_hash, &api_key_data)
        .map_err(|err| StdError::generic_err(err.to_string()))?;

    // For the response, we might *not* want to reveal the hash in events (up to you).
    // But we'll do it here for illustration.
    Ok(Response::new()
        .add_attribute("action", "add_api_key")
        .add_attribute("stored_hash", key_hash))
}

pub fn try_revoke_api_key(
    deps: DepsMut,
    info: MessageInfo,
    api_key: String,
) -> StdResult<Response> {
    let config = config_read(deps.storage);
    let state = config.load()?;

    // Check if the sender is the admin
    if info.sender != state.admin {
        return Err(StdError::generic_err("Only admin can revoke API keys"));
    }

    // 1. Compute the hash again
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    // 2. Check if this hash is in storage
    if !API_KEY_MAP.contains(deps.storage, &key_hash) {
        return Err(StdError::generic_err("API key (hash) not found"));
    }

    // 3. Remove the entry
    API_KEY_MAP
        .remove(deps.storage, &key_hash)
        .map_err(|err| StdError::generic_err(err.to_string()))?;

    // Return a response
    Ok(Response::new()
        .add_attribute("action", "revoke_api_key")
        .add_attribute("removed_hash", key_hash))
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    match msg {
        MigrateMsg::Migrate {} => {
            // Collect all keys using `iter_keys`
            let keys_to_remove: Vec<String> = API_KEY_MAP
                .iter_keys(deps.storage)?
                .filter_map(|key_result| key_result.ok())
                .collect();

            // Remove each key
            for key in keys_to_remove {
                API_KEY_MAP.remove(deps.storage, &key)?;
            }

            Ok(Response::new()
                .add_attribute("action", "migrate")
                .add_attribute("status", "api_key_map_cleared"))
        }
        MigrateMsg::StdError {} => Err(StdError::generic_err("this is an std error")),
    }
}

// Function to register a new subscriber
pub fn try_register_subscriber(
    deps: DepsMut,
    info: MessageInfo,
    public_key: String,
) -> StdResult<Response> {
    // Check if the sender is the admin
    let config = config_read(deps.storage);
    let state = config.load()?;
    if info.sender != state.admin {
        return Err(StdError::generic_err("Only admin can register subscribers"));
    }

    // Check if the subscriber is already registered
    let map_contains_sb = SB_MAP.contains(deps.storage, &public_key);
    if map_contains_sb {
        return Err(StdError::generic_err("Subscriber already registered"));
    }

    // Create a new subscriber and insert it into the map
    let subscriber = Subscriber { status: true };
    SB_MAP.insert(deps.storage, &public_key, &subscriber)
        .map_err(|err| StdError::generic_err(err.to_string()))?;

    // Return a response indicating successful registration
    Ok(Response::new()
        .add_attribute("action", "register_subscriber")
        .add_attribute("subscriber", public_key))
}

// Function to remove a subscriber
pub fn try_remove_subscriber(
    deps: DepsMut,
    info: MessageInfo,
    public_key: String,
) -> StdResult<Response> {
    // Check if the sender is the admin
    let config = config_read(deps.storage);
    let state = config.load()?;
    if info.sender != state.admin {
        return Err(StdError::generic_err("Only admin can remove subscribers"));
    }

    // Check if the subscriber is registered
    let map_contains_sb = SB_MAP.contains(deps.storage, &public_key);
    if !map_contains_sb {
        return Err(StdError::generic_err("Subscriber not registered"));
    }

    // Remove the subscriber from the map
    SB_MAP.remove(deps.storage, &public_key)
        .map_err(|err| StdError::generic_err(err.to_string()))?;

    // Return a response indicating successful removal
    Ok(Response::new()
        .add_attribute("action", "remove_subscriber")
        .add_attribute("subscriber", public_key))
}

// Function to set a new admin
pub fn try_set_admin(deps: DepsMut, info: MessageInfo, public_key: String) -> StdResult<Response> {
    let mut config = config(deps.storage);
    let mut state = config.load()?;

    // Check if the sender is the current admin
    if info.sender != state.admin {
        return Err(StdError::generic_err("Only the current admin can set a new admin"));
    }

    // Validate the new admin's public key
    let final_address = deps.api.addr_validate(&public_key).map_err(|err| {
        StdError::generic_err(format!("Invalid address: {}", err))
    })?;

    // Update the admin in the state
    state.admin = final_address;
    config.save(&state)?;

    // Return a response indicating successful admin update
    Ok(Response::new()
        .add_attribute("action", "set_admin")
        .add_attribute("new_admin", public_key))
}

// Entry point for handling queries
#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::SubscriberStatusWithPermit { public_key, permit } => {
            to_binary(&query_subscriber_with_permit(deps, env, public_key, permit)?)
        }
        QueryMsg::GetAdmin {} => to_binary(&get_admin(deps)?),
        QueryMsg::ApiKeysWithPermit { permit } => {
            to_binary(&query_api_keys_with_permit(deps, env, permit)?)
        }
    }
}

// Function to get the current admin
fn get_admin(deps: Deps) -> StdResult<Addr> {
    let state = config_read(deps.storage).load()?;
    Ok(state.admin)
}

// Function to check if a subscriber is active
fn query_subscriber_with_permit(
    deps: Deps,
    env: Env,
    public_key: String,
    permit: Permit,
) -> StdResult<SubscriberStatusResponse> {
    // 1. Read current admin from contract state
    let state = config_read(deps.storage).load()?;
    let admin_addr = state.admin;

    //  Validate permit name
    if permit.params.permit_name != "query_subscriber_permit" {
        return Err(StdError::generic_err("Invalid permit name"));
    }

    // 2. Validate the permit
    let contract_address = env.contract.address;
    let storage_prefix = "permits_subscriber_status";
    let signer_addr = validate(
        deps,
        storage_prefix,
        &permit,
        contract_address.into_string(),
        Some("secret"),
    )?;

    // 3. Check if the signer is actually the admin
    if signer_addr != admin_addr {
        return Err(StdError::generic_err("Unauthorized: not the admin"));
    }

    // 4. Check if the subscriber exists
    let subscriber = SB_MAP.get(deps.storage, &public_key);
    let active = subscriber.is_some();

    Ok(SubscriberStatusResponse { active })
}

/// Validates the permit and, if valid and signed by the admin, returns all API keys
fn query_api_keys_with_permit(
    deps: Deps,
    env: Env,
    permit: Permit,
) -> StdResult<GetApiKeysResponse> {
    // 1. Read current admin from contract state
    let state = config_read(deps.storage).load()?;
    let admin_addr = state.admin; // e.g. "secret1xyz..."

    //  Validate permit name
    if permit.params.permit_name != "api_keys_permit" {
        return Err(StdError::generic_err("Invalid permit name"));
    }

    // 2. Convert our contract address to `HumanAddr` (if needed by validate)
    //    Some validate methods require the "contract_address" or similar.
    //    In many SNIP-20 references, the "contract_address" is just the
    //    contract address itself, because you typically check that
    //    permit.params.allowed_tokens includes this contract.
    let contract_address = env.contract.address;

    // 3. storage_prefix is the prefix in storage for revoked permits (if used).
    //    Typically something like "permits" or "revoke_permits".
    let storage_prefix = "permits_api_keys";

    // 4. Validate the permit
    //    This should check:
    //      - The signature is correct
    //      - The permit has not been revoked
    //      - The contract address is in `allowed_tokens` (if you require that)
    //
    //    In your snippet, `validate` returns the signer's bech32 address
    //    if the signature is valid, or an error otherwise.

    let signer_addr = validate(
        deps,
        storage_prefix,
        &permit,
        contract_address.into_string(),
        Some("secret"), // The HRP, e.g. "secret", "cosmos", etc.
    )?;

    // 5. Check if the signer is actually the admin
    if signer_addr != admin_addr {
        return Err(StdError::generic_err("Unauthorized: not the admin"));
    }

    // 4. Use `iter_keys` to construct `ApiKeyResponse` directly from the keys
    let api_keys: Vec<ApiKeyResponse> = API_KEY_MAP
        .iter_keys(deps.storage)? // Iterate over the keys
        .filter_map(|key_result| {
            if let Ok(key) = key_result {
                // Construct `ApiKeyResponse` from the key
                Some(ApiKeyResponse {
                    hashed_key: key,
                })
            } else {
                None // Skip invalid keys
            }
        })
        .collect();

    Ok(GetApiKeysResponse { api_keys })
}
#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::*;
    use cosmwasm_std::{attr, from_binary, BlockInfo, Coin, ContractInfo, Timestamp, TransactionInfo, Uint128};

    fn mock_env_for_permit() -> Env {
        let env = Env {
            block: BlockInfo {
                height: 12_345,
                time: Timestamp::from_nanos(1_571_797_419_879_305_533),
                chain_id: "pulsar-3".to_string(),
                random: Some(
                    Binary::from_base64("wLsKdf/sYqvSMI0G0aWRjob25mrIB0VQVjTjDXnDafk=").unwrap(),
                ),
            },
            transaction: Some(TransactionInfo {
                index: 3,
                hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
            }),
            contract: ContractInfo {
                address: Addr::unchecked("secret1ttm9axv8hqwjv3qxvxseecppsrw4cd68getrvr"),
                code_hash: "".to_string(),
            },
        };
        env
    }

    #[test]
    fn test_migrate_clears_api_key_map() {
        let mut deps = mock_dependencies();

        // Initialize the contract with an admin address
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // Add API keys to the `API_KEY_MAP`
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::AddApiKey {
                api_key: "test_key1".to_string(),
            },
        )
            .unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::AddApiKey {
                api_key: "test_key2".to_string(),
            },
        )
            .unwrap();

        // Ensure that the keys were added successfully
        let keys: Vec<String> = API_KEY_MAP
            .iter_keys(deps.as_ref().storage)
            .unwrap()
            .filter_map(|key_result| key_result.ok())
            .collect();
        assert_eq!(keys.len(), 2);

        // Perform migration
        migrate(deps.as_mut(), mock_env(), MigrateMsg::Migrate {}).unwrap();

        // Ensure the keys are removed
        let keys_after_migration: Vec<String> = API_KEY_MAP
            .iter_keys(deps.as_ref().storage)
            .unwrap()
            .filter_map(|key_result| key_result.ok())
            .collect();
        assert!(keys_after_migration.is_empty());
    }

    #[test]
    fn test_query_api_keys_with_real_permit() {
        // 1. Initialize the contract with admin = "secret1p55wr2n6f63wyap8g9dckkxmf4wvq73ensxrw4"
        let mut deps = mock_dependencies();
        let info = mock_info("secret1p55wr2n6f63wyap8g9dckkxmf4wvq73ensxrw4", &[]);
        let init_msg = InstantiateMsg {};

        // Create a custom Env if you need specific block/transaction data
        let env = mock_env_for_permit();

        // Instantiate the contract
        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        // 2. Add a test API key so we can verify it during the query
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::AddApiKey {
                api_key: "test_key1".to_string(),
            },
        )
            .unwrap();

        // 3. Read the permit from a file (e.g., "./permit.json").
        //    This JSON should be a properly signed permit (StdSignDoc + signature),
        //    or a directly "cleaned" JSON that matches secret_toolkit::permit::Permit.
        let json_data = std::fs::read_to_string("./api_keys_permit.json").unwrap();
        let permit: Permit = serde_json::from_str(&json_data)
            .expect("Could not parse Permit from JSON");

        // 4. Query the contract using the permit
        let query_msg = QueryMsg::ApiKeysWithPermit { permit };
        println!("Query_msg: {:#?}", query_msg);
        let res = query(deps.as_ref(), env.clone(), query_msg);

        // 5. Check the response to ensure the API key is returned
        match res {
            Ok(bin) => {
                let parsed: GetApiKeysResponse = from_binary(&bin).unwrap();
                // We expect exactly 1 API key: "test_key1"
                assert_eq!(parsed.api_keys.len(), 1);
                println!("Response: {:#?}", parsed);
            }
            Err(e) => panic!("Query failed: {:?}", e),
        }
    }

    #[test]
    fn revoke_api_key_and_query_with_permit() {
        // 1. Initialize the contract with some admin address
        let mut deps = mock_dependencies();
        // Suppose "admin" is just a placeholder address (like "secret1abc...")
        let info = mock_info("secret1p55wr2n6f63wyap8g9dckkxmf4wvq73ensxrw4", &[]);
        let init_msg = InstantiateMsg {};

        // Create a custom Env if you need specific block/transaction data
        let env = mock_env_for_permit();

        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        // 2. Add an API key
        let add_msg = ExecuteMsg::AddApiKey {
            api_key: "test_api_key".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), add_msg).unwrap();

        // 3. Revoke (remove) that API key
        let revoke_msg = ExecuteMsg::RevokeApiKey {
            api_key: "test_api_key".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), revoke_msg).unwrap();

        // 4. Now load a real signed Permit from file (as in your `test_query_api_keys_with_real_permit`)
        //    This permit must be signed by the same admin address in order to pass validation.
        let json_data = std::fs::read_to_string("./api_keys_permit.json")
            .expect("Failed to read permit.json");
        let permit: secret_toolkit::permit::Permit = serde_json::from_str(&json_data)
            .expect("Could not parse Permit from JSON");

        // 5. Perform a query that uses the permit
        //    This calls your existing `ApiKeysWithPermit { permit }` query
        let query_msg = QueryMsg::ApiKeysWithPermit { permit };
        let res = query(deps.as_ref(), env.clone(), query_msg)
            .expect("Query failed unexpectedly");

        // 6. Verify that the revoked key is no longer in the list
        let response: GetApiKeysResponse = from_binary(&res).unwrap();
        assert!(
            response.api_keys.is_empty(),
            "Expected empty API keys after revoke, got: {:?}",
            response.api_keys
        );

        println!("Revoke test passed. 'test_api_key' is no longer in the list.");
    }

    #[test]
    /// Test for successful initialization of the contract
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let info = mock_info(
            "creator",
            &[Coin {
                denom: "earth".to_string(),
                amount: Uint128::new(1000),
            }],
        );
        let init_msg = InstantiateMsg {};

        // Assert successful initialization
        let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    /// Test successful registration of a subscriber by admin
    fn register_subscriber_success() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let register_msg = ExecuteMsg::RegisterSubscriber {
            public_key: "subscriber1".to_string(),
        };

        // Execute the message to register the subscriber and check the response
        let res = execute(deps.as_mut(), mock_env(), info, register_msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "register_subscriber"),
                attr("subscriber", "subscriber1")
            ]
        );
    }

    #[test]
    /// Test registration attempt by a non-admin, expecting failure
    fn register_subscriber_unauthorized() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        let unauthorized_info = mock_info("not_admin", &[]);
        let register_msg = ExecuteMsg::RegisterSubscriber {
            public_key: "subscriber1".to_string(),
        };

        // Attempt to register with a non-admin account and expect an error
        let res = execute(deps.as_mut(), mock_env(), unauthorized_info, register_msg);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            StdError::generic_err("Only admin can register subscribers")
        );
    }

    #[test]
    /// Test successful removal of a subscriber by admin
    fn remove_subscriber_success() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // Register a subscriber first
        let register_msg = ExecuteMsg::RegisterSubscriber {
            public_key: "subscriber1".to_string(),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), register_msg).unwrap();

        // Now remove the subscriber
        let remove_msg = ExecuteMsg::RemoveSubscriber {
            public_key: "subscriber1".to_string(),
        };

        // Execute the message to remove the subscriber and check the response
        let res = execute(deps.as_mut(), mock_env(), info, remove_msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "remove_subscriber"),
                attr("subscriber", "subscriber1")
            ]
        );
    }

    #[test]
    /// Test removal attempt of a non-registered subscriber, expecting failure
    fn remove_subscriber_not_registered() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let remove_msg = ExecuteMsg::RemoveSubscriber {
            public_key: "subscriber1".to_string(),
        };

        // Attempt to remove a non-registered subscriber and expect an error
        let res = execute(deps.as_mut(), mock_env(), info, remove_msg);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            StdError::generic_err("Subscriber not registered")
        );
    }

    #[test]
    /// Test successful update of the admin by the current admin
    fn set_admin_success() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let set_admin_msg = ExecuteMsg::SetAdmin {
            public_key: "new_admin".to_string(),
        };

        // Execute the message to set a new admin and check the response
        let res = execute(deps.as_mut(), mock_env(), info, set_admin_msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "set_admin"),
                attr("new_admin", "new_admin")
            ]
        );

        // Check that the admin was updated successfully
        let config = config_read(&deps.storage).load().unwrap();
        assert_eq!(config.admin, Addr::unchecked("new_admin"));
    }

    #[test]
    /// Test admin update attempt by a non-admin, expecting failure
    fn set_admin_unauthorized() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        let unauthorized_info = mock_info("not_admin", &[]);
        let set_admin_msg = ExecuteMsg::SetAdmin {
            public_key: "new_admin".to_string(),
        };

        // Attempt to set a new admin with a non-admin account and expect an error
        let res = execute(deps.as_mut(), mock_env(), unauthorized_info, set_admin_msg);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            StdError::generic_err("Only the current admin can set a new admin")
        );
    }

    #[test]
    fn test_get_admin() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let query_msg = QueryMsg::GetAdmin {};
        let bin = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let response: Addr = from_binary(&bin).unwrap();

        println!("Response: {:#?}", response);

        assert_eq!(response, Addr::unchecked("admin"));
    }

    #[test]
    /// Test querying for a registered subscriber, expecting active status
    fn query_registered_subscriber() {
        // 1. Initialize the contract with some admin address
        let mut deps = mock_dependencies();
        // Suppose "admin" is just a placeholder address (like "secret1abc...")
        let info = mock_info("secret1p55wr2n6f63wyap8g9dckkxmf4wvq73ensxrw4", &[]);
        let init_msg = InstantiateMsg {};

        // Create a custom Env if you need specific block/transaction data
        let env = mock_env_for_permit();

        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        // Register a subscriber
        let register_msg = ExecuteMsg::RegisterSubscriber {
            public_key: "subscriber_public_key".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info, register_msg).unwrap();

        let json_data = std::fs::read_to_string("./query_subscriber_permit.json")
            .expect("Failed to read permit.json");
        let permit: secret_toolkit::permit::Permit = serde_json::from_str(&json_data)
            .expect("Could not parse Permit from JSON");

        // Query for the registered subscriber and check the response
        let query_msg = QueryMsg::SubscriberStatusWithPermit {
            public_key: "subscriber_public_key".to_string(),
            permit: permit.clone(),
        };
        let bin = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let response: SubscriberStatusResponse = from_binary(&bin).unwrap();

        println!("Response: {:#?}", response);

        // Check that the subscriber is active
        assert!(response.active);
    }

    #[test]
    /// Test querying for an unregistered subscriber, expecting inactive status
    fn query_unregistered_subscriber() {
        // 1. Initialize the contract with some admin address
        let mut deps = mock_dependencies();
        // Suppose "admin" is just a placeholder address (like "secret1abc...")
        let info = mock_info("secret1p55wr2n6f63wyap8g9dckkxmf4wvq73ensxrw4", &[]);
        let init_msg = InstantiateMsg {};

        // Create a custom Env if you need specific block/transaction data
        let env = mock_env_for_permit();
        instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

        let json_data = std::fs::read_to_string("./query_subscriber_permit.json")
            .expect("Failed to read permit.json");
        let permit: secret_toolkit::permit::Permit = serde_json::from_str(&json_data)
            .expect("Could not parse Permit from JSON");

        // Query for an unregistered subscriber and check the response
        let query_msg = QueryMsg::SubscriberStatusWithPermit {
            public_key: "unregistered_public_key".to_string(),
            permit: permit.clone(),
        };
        let bin = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let response: SubscriberStatusResponse = from_binary(&bin).unwrap();

        // Check that the subscriber is not active
        assert!(!response.active);
    }

    #[test]
    /// Test querying for a subscriber after removal, expecting inactive status
    fn query_subscriber_after_removal() {
        // 1. Initialize the contract with some admin address
        let mut deps = mock_dependencies();
        // Suppose "admin" is just a placeholder address (like "secret1abc...")
        let info = mock_info("secret1p55wr2n6f63wyap8g9dckkxmf4wvq73ensxrw4", &[]);
        let init_msg = InstantiateMsg {};

        // Create a custom Env if you need specific block/transaction data
        let env = mock_env_for_permit();

        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        // Register a subscriber
        let register_msg = ExecuteMsg::RegisterSubscriber {
            public_key: "subscriber_public_key".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), register_msg).unwrap();

        // Remove the subscriber
        let remove_msg = ExecuteMsg::RemoveSubscriber {
            public_key: "subscriber_public_key".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info, remove_msg).unwrap();

        let json_data = std::fs::read_to_string("./query_subscriber_permit.json")
            .expect("Failed to read permit.json");
        let permit: secret_toolkit::permit::Permit = serde_json::from_str(&json_data)
            .expect("Could not parse Permit from JSON");

        // Query for the subscriber after removal and check the response
        let query_msg = QueryMsg::SubscriberStatusWithPermit {
            public_key: "subscriber_public_key".to_string(),
            permit: permit.clone(),
        };
        let bin = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let response: SubscriberStatusResponse = from_binary(&bin).unwrap();

        // Check that the subscriber is not active
        assert!(!response.active);
    }

}