#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, GetCw20AddressResponse, GetOwnerResponse, GetWithdrawBalanceResponse,
    InstantiateMsg, QueryMsg,
};
use crate::state::{State, STATE, WITHDRAW_BALANCES};

use cw20::{Cw20Contract, Cw20ExecuteMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-contract-sample";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender.clone(),
        cw20_addr: deps.api.addr_validate(msg.cw20_addr.as_str())?,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("cw20_addr", msg.cw20_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SendCoinsToContract {
            amount,
            cw20_addr,
            recipient1,
            recipient2,
        } => send_coins(deps, _env, info, amount, cw20_addr, recipient1, recipient2),
        ExecuteMsg::WithdrawCoinsFromContract { amount, cw20_addr } => {
            withdraw_coins(deps, _env, info, amount, cw20_addr)
        }
    }
}

// Send coins to contract - user can specify 2 recipients
pub fn send_coins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    cw20_addr: String,
    recipient1: String,
    recipient2: String,
) -> Result<Response, ContractError> {
    // TODO: add require check that this cw20_addr is equal to one in state
    // Conduct cw20 transfer to send funds from msg sender to contract
    let cw20 = Cw20Contract(Addr::unchecked(cw20_addr));
    let msg = cw20.call(Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.to_string(),
        recipient: _env.contract.address.into_string(),
        amount: amount,
    })?;

    // Calculate split amount - TODO: check moved value
    let split_amount = amount.checked_div(Uint128::new(2));
    let split_amount2 = amount.checked_div(Uint128::new(2));

    // Add withdraw balance to both recipients with split amount
    WITHDRAW_BALANCES.update(
        deps.storage,
        &Addr::unchecked(&recipient1),
        |withdraw_balance: Option<Uint128>| -> StdResult<_> {
            Ok(withdraw_balance.unwrap_or_default() + split_amount.unwrap_or_default())
        },
    )?;
    WITHDRAW_BALANCES.update(
        deps.storage,
        &Addr::unchecked(&recipient2),
        |withdraw_balance: Option<Uint128>| -> StdResult<_> {
            Ok(withdraw_balance.unwrap_or_default() + split_amount2.unwrap_or_default())
        },
    )?;

    let mut res = Response::new()
        .add_attribute("action", "sendCoins")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("recipient1", Addr::unchecked(&recipient1))
        .add_attribute("recipient1", Addr::unchecked(&recipient2));
    res = res.add_message(msg);
    Ok(res)
}

// Withdraw coins, up to amount
pub fn withdraw_coins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    cw20_addr: String,
) -> Result<Response, ContractError> {
    // Check that amount does not exceed withdraw balance
    let withdraw_balance = WITHDRAW_BALANCES
        .load(deps.storage, &Addr::unchecked(&info.sender))
        .unwrap_or_default();
    if withdraw_balance < amount {
        return Err(ContractError::WithdrawAmountExceedsBalance {});
    }
    // TODO: add require check that this cw20_addr is equal to one in state
    // Conduct cw20 transfer from contract to msg sender
    let cw20 = Cw20Contract(Addr::unchecked(cw20_addr));
    let msg = cw20.call(Cw20ExecuteMsg::TransferFrom {
        owner: _env.contract.address.into_string(),
        recipient: info.sender.to_string(),
        amount: amount,
    })?;

    // Reduce withdraw balance for msg sender
    WITHDRAW_BALANCES.update(
        deps.storage,
        &Addr::unchecked(&info.sender),
        |withdraw_balance: Option<Uint128>| -> StdResult<_> {
            Ok(withdraw_balance.unwrap_or_default() - amount)
        },
    )?;

    let mut res = Response::new()
        .add_attribute("action", "withdrawCoins")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount);
    res = res.add_message(msg);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner {} => to_binary(&query_owner(deps)?),
        QueryMsg::GetCw20Address {} => to_binary(&query_cw20_addr(deps)?),
        QueryMsg::GetWithdrawBalance { recipient } => {
            to_binary(&query_withdraw_balance(deps, recipient)?)
        }
    }
}

// Query owner
fn query_owner(deps: Deps) -> StdResult<GetOwnerResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(GetOwnerResponse { owner: state.owner })
}

// Query cw20 addr
fn query_cw20_addr(deps: Deps) -> StdResult<GetCw20AddressResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(GetCw20AddressResponse {
        cw20_addr: state.cw20_addr,
    })
}

// Query withdraw balance for recipient
fn query_withdraw_balance(deps: Deps, recipient: String) -> StdResult<GetWithdrawBalanceResponse> {
    let withdraw_balance = WITHDRAW_BALANCES.load(deps.storage, &Addr::unchecked(recipient));
    Ok(GetWithdrawBalanceResponse {
        withdraw_balance: withdraw_balance.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn create_contract() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let owner = String::from("owner");
        let info = mock_info(&owner, &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // query owner
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOwner {}).unwrap();
        let value: GetOwnerResponse = from_binary(&res).unwrap();
        assert_eq!(&owner, &value.owner);

        // query cw20 address
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCw20Address {}).unwrap();
        let value: GetCw20AddressResponse = from_binary(&res).unwrap();
        assert_eq!(&String::from(MOCK_CONTRACT_ADDR), &value.cw20_addr);
    }

    #[test]
    fn send_coins() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let mut info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let recipient1 = String::from("recipient1");
        let recipient2 = String::from("recipient2");

        // Send coins to contract
        let msg = ExecuteMsg::SendCoinsToContract {
            amount: Uint128::new(100),
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
            recipient1: recipient1,
            recipient2: recipient2,
        };
        info.sender = Addr::unchecked("cw20");
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Withdraw balances should be 50 for both recipients
        let res1 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetWithdrawBalance {
                recipient: String::from("recipient1"),
            },
        )
        .unwrap();
        let value: GetWithdrawBalanceResponse = from_binary(&res1).unwrap();
        assert_eq!(Uint128::new(50), value.withdraw_balance);

        let res2 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetWithdrawBalance {
                recipient: String::from("recipient2"),
            },
        )
        .unwrap();
        let value: GetWithdrawBalanceResponse = from_binary(&res2).unwrap();
        assert_eq!(Uint128::new(50), value.withdraw_balance);
    }

    #[test]
    fn withdraw_coins() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let mut info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Withdraw balance is 0
        let res0 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetWithdrawBalance {
                recipient: String::from("random"),
            },
        )
        .unwrap();
        let value: GetWithdrawBalanceResponse = from_binary(&res0).unwrap();
        assert_eq!(Uint128::new(0), value.withdraw_balance);

        // So an attempt to withdraw 100 should fail
        let msg = ExecuteMsg::WithdrawCoinsFromContract {
            amount: Uint128::new(100),
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let random_info = mock_info("random", &coins(2, "token"));
        let res = execute(deps.as_mut(), mock_env(), random_info, msg);
        match res {
            Err(ContractError::WithdrawAmountExceedsBalance {}) => {}
            _ => panic!("Must return withdraw exceeds amount balance"),
        }

        // Similar setup to before, send coins to two recipients
        let recipient1 = String::from("recipient1");
        let recipient2 = String::from("recipient2");
        let msg = ExecuteMsg::SendCoinsToContract {
            amount: Uint128::new(100),
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
            recipient1: recipient1,
            recipient2: recipient2,
        };
        info.sender = Addr::unchecked("cw20");
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Recipient 1 withdraws 30, should have 20 left in withdrawal balance
        let msg = ExecuteMsg::WithdrawCoinsFromContract {
            amount: Uint128::new(30),
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let recipient1_info = mock_info("recipient1", &coins(2, "token"));
        let _res = execute(deps.as_mut(), mock_env(), recipient1_info.clone(), msg);

        let res1 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetWithdrawBalance {
                recipient: String::from("recipient1"),
            },
        )
        .unwrap();
        let value: GetWithdrawBalanceResponse = from_binary(&res1).unwrap();
        assert_eq!(Uint128::new(20), value.withdraw_balance);
    }
}
