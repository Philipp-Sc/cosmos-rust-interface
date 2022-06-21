/*
 * terra-rust-api utilitiy functions to estimate/execute transactions.
 *
 * https://docs.rs/cw20/0.8.0/cw20/enum.Cw20ExecuteMsg.html
 * https://github.com/Anchor-Protocol
 * https://github.com/astroport-fi/astroport-core/
 * https://github.com/spectrumprotocol/contracts/
 */

pub mod api;

use api::{execute_messages, estimate_messages, estimate_to_gas_opts};


use terra_rust_api::core_types::{Coin};
use terra_rust_api::{PrivateKey};
use terra_rust_api::messages::wasm::MsgExecuteContract;
use terra_rust_api::messages::Message;

use secp256k1::Secp256k1;
use rust_decimal::Decimal;
use core::str::FromStr;
use anyhow::anyhow;


use rust_decimal::prelude::ToPrimitive;
use cosmwasm_bignumber::{Uint256};
use cosmwasm_std_deprecated::{to_binary, Uint128};

//use cosmwasm_std_deprecated::Binary;
// Binary::from_base64(&base64::encode(msg))?

use cw20::Cw20ExecuteMsg;
use moneymarket::market::ExecuteMsg;
use terraswap::asset::{Asset, AssetInfo};

use anchor_token::airdrop::ExecuteMsg as AirdropExecuteMsg;
use api::data::terra_contracts::{contracts, tokens, custom};

use api::data::terra_contracts::AssetWhitelist;
use std::sync::Arc;

// todo: swap messages!

fn astroport_swap_msg(asset_whitelist: &Arc<AssetWhitelist>, wallet_acc_address: &str, coin_amount: Decimal, max_spread: Decimal, belief_price: Decimal) -> anyhow::Result<Message> {
    let contract_addr_anc = tokens(asset_whitelist, "Anchor", "ANC").ok_or(anyhow!("no contract_addr_anc"))?;
    let contract_addr_lp = custom(asset_whitelist, "Anchor", "ANC-UST LP Minter").ok_or(anyhow!("no contract_addr_lp"))?;
    let coins: [Coin; 0] = []; // no coins needed

    let msg = astroport::pair::Cw20HookMsg::Swap {
        belief_price: Some(cosmwasm_std_deprecated::Decimal::from_str(belief_price.round_dp_with_strategy(18, rust_decimal::RoundingStrategy::ToZero).to_string().as_str())?),
        max_spread: Some(cosmwasm_std_deprecated::Decimal::from_str(max_spread.to_string().as_str())?),
        to: None,
    };

    let execute_msg = Cw20ExecuteMsg::Send {
        contract: contract_addr_lp,
        amount: Uint128::from(coin_amount.to_u128().ok_or(anyhow!("incorrect coin_amount format"))?),
        msg: to_binary(&msg).unwrap(),
    };
    let execute_msg_json = serde_json::to_string(&execute_msg)?;

    let send = MsgExecuteContract::create_from_json(&wallet_acc_address, &contract_addr_anc, &execute_msg_json, &coins)?;
    return Ok(send);
}
