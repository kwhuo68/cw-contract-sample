use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub cw20_addr: Addr,
}

pub const STATE: Item<State> = Item::new("state");
pub const WITHDRAW_BALANCES: Map<&Addr, Uint128> = Map::new("withdraw_balance");
