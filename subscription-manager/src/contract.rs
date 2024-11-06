use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult
};
use secp256k1::{Message, PublicKey, Secp256k1, ecdsa::Signature};
use sha2::{Digest, Sha256};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SubscriberStatusResponse};
use crate::state::{config, config_read, State, Subscriber, SB_MAP};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        admin: info.sender.clone(),
    };

    deps.api
        .debug(format!("Contract was initialized by {}", info.sender).as_str());
    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::RegisterSubscriber { address } => try_register_subscriber(deps, info, address),
        ExecuteMsg::RemoveSubscriber { address } => try_remove_subscriber(deps, info, address),
        ExecuteMsg::SetAdmin { address } => try_set_admin(deps, info, address),
    }
}

pub fn try_register_subscriber(
    _deps: DepsMut,
    _info: MessageInfo,
    _address: String,
) -> StdResult<Response> {

    let config = config_read(_deps.storage);
    let state = config.load()?;
    if _info.sender != state.admin {
        return Err(StdError::generic_err("Only admin can register subscribers"));
    }

    let map_contains_sb = SB_MAP.contains(_deps.storage, &_address);

    if map_contains_sb {
        return Err(StdError::generic_err("Subscriber already registered"));
    }

    let subscriber = Subscriber { address: _address.clone(), status: true };
        // Insert new value
    
    SB_MAP.insert(_deps.storage, &_address, &subscriber)
        .map_err(|err| StdError::generic_err(err.to_string()))?;

    Ok(Response::new()
        .add_attribute("action", "register_subscriber")
        .add_attribute("subscriber", _address))
}

pub fn try_remove_subscriber(
    _deps: DepsMut,
    _info: MessageInfo,
    _address: String,
) -> StdResult<Response> {
    let config = config_read(_deps.storage);
    let state = config.load()?;
    if _info.sender != state.admin {
        return Err(StdError::generic_err("Only admin can remove subscribers"));
    }

    let map_contains_sb = SB_MAP.contains(_deps.storage, &_address);

    if !map_contains_sb {
        return Err(StdError::generic_err("Subscriber not registered"));
    }

    SB_MAP.remove(_deps.storage, &_address)
        .map_err(|err| StdError::generic_err(err.to_string()))?;

    Ok(Response::new()
        .add_attribute("action", "remove_subscriber")
        .add_attribute("subscriber", _address))
}

pub fn try_set_admin(_deps: DepsMut, _info: MessageInfo, _address: String) -> StdResult<Response> {
    let mut config = config(_deps.storage);
    let mut state = config.load()?;

    // Only the current admin can set a new admin
    if _info.sender != state.admin {
        return Err(StdError::generic_err("Only the current admin can set a new admin"));
    }

    // let canonical_address = _deps.api.addr_canonicalize(&_address)
    // .map_err(|err| {
    //     StdError::generic_err(format!("Invalid address: {}", err))
    // });

    // if canonical_address.is_err() {
    //     return Err(StdError::generic_err("Invalid address"));
    // }

    // let final_address = _deps.api.addr_humanize(&canonical_address.unwrap());

    // if final_address.is_err() {
    //     return Err(StdError::generic_err("Invalid address"));
    // }

    let final_address = _deps.api.addr_validate(&_address).map_err(|err| {
        StdError::generic_err(format!("Invalid address: {}", err))
    })?;

    // Update the admin to the new address
    state.admin = final_address;
    config.save(&state)?;

    // Log the admin change
    Ok(Response::new()
        .add_attribute("action", "set_admin")
        .add_attribute("new_admin", _address))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::SubscriberStatus {
            address,
            signature,
            sender_public_key,
        } => to_binary(&query_subscriber(
            deps,
            address,
            signature,
            sender_public_key,
        )?),
    }
}

fn query_subscriber(
    _deps: Deps,
    _address: String,
    _signature: String,
    _sender_public_key: String,
) -> StdResult<SubscriberStatusResponse> {

    let payload = format!("{}{}", _address, "_payload_message");

    let is_valid = verify_signature(_sender_public_key, _signature, payload.as_bytes())?;

    if !is_valid {
        return Err(StdError::generic_err("Invalid signature"));
    }

    let subscriber = SB_MAP.get(_deps.storage, &_address);

    if !subscriber.is_none() {
        return Ok(SubscriberStatusResponse { active: true });
    }

    Ok(SubscriberStatusResponse { active: false })
}

fn verify_signature(
    public_key_hex: String,
    signature_hex: String,
    message: &[u8],
) -> StdResult<bool> {
    let secp = Secp256k1::verification_only();

    let public_key_bytes = hex::decode(public_key_hex)
        .map_err(|_| StdError::generic_err("Invalid public key hex"))?;
    let public_key = PublicKey::from_slice(&public_key_bytes)
        .map_err(|_| StdError::generic_err("Invalid public key"))?;

    let signature_bytes = hex::decode(signature_hex)
        .map_err(|_| StdError::generic_err("Invalid signature hex"))?;
    let signature = Signature::from_der(&signature_bytes)
        .map_err(|_| StdError::generic_err("Invalid signature"))?;

    let message_hash = sha2::Sha256::digest(message);
    let message = Message::from_slice(&message_hash)
        .map_err(|_| StdError::generic_err("Invalid message"))?;

    secp.verify_ecdsa(&message, &signature, &public_key)
        .map(|_| true)
        .map_err(|_| StdError::generic_err("Signature verification failed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::*;
    use cosmwasm_std::{attr, from_binary, Coin, Uint128};

    #[test]
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
    fn register_subscriber_success() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let register_msg = ExecuteMsg::RegisterSubscriber {
            address: "subscriber1".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), info, register_msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(res.attributes, vec![
            attr("action", "register_subscriber"),
            attr("subscriber", "subscriber1")
        ]);
    }

    #[test]
    fn register_subscriber_unauthorized() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        let unauthorized_info = mock_info("not_admin", &[]);
        let register_msg = ExecuteMsg::RegisterSubscriber {
            address: "subscriber1".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), unauthorized_info, register_msg);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            StdError::generic_err("Only admin can register subscribers")
        );
    }

    #[test]
    fn remove_subscriber_success() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // Register a subscriber first
        let register_msg = ExecuteMsg::RegisterSubscriber {
            address: "subscriber1".to_string(),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), register_msg).unwrap();

        // Now remove the subscriber
        let remove_msg = ExecuteMsg::RemoveSubscriber {
            address: "subscriber1".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, remove_msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(res.attributes, vec![
            attr("action", "remove_subscriber"),
            attr("subscriber", "subscriber1")
        ]);
    }

    #[test]
    fn remove_subscriber_not_registered() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let remove_msg = ExecuteMsg::RemoveSubscriber {
            address: "subscriber1".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, remove_msg);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            StdError::generic_err("Subscriber not registered")
        );
    }

    #[test]
    fn set_admin_success() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let set_admin_msg = ExecuteMsg::SetAdmin {
            address: "new_admin".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), info, set_admin_msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(res.attributes, vec![
            attr("action", "set_admin"),
            attr("new_admin", "new_admin")
        ]);

        // Check that the admin was updated
        let config = config_read(&deps.storage).load().unwrap();
        assert_eq!(config.admin, Addr::unchecked("new_admin"));
    }

    #[test]
    fn set_admin_unauthorized() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let init_msg = InstantiateMsg {};
        instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        let unauthorized_info = mock_info("not_admin", &[]);
        let set_admin_msg = ExecuteMsg::SetAdmin {
            address: "new_admin".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), unauthorized_info, set_admin_msg);
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            StdError::generic_err("Only the current admin can set a new admin")
        );
    }
}