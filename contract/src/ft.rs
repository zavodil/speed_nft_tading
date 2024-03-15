use crate::*;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::json_types::U128;
use near_sdk::{is_promise_success, serde_json, Promise, PromiseError, PromiseOrValue, PromiseResult};

const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(10);
const GAS_FOR_AFTER_FT_TRANSFER: Gas = Gas::from_tgas(20);
pub const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas::from_tgas(30);
pub const GAS_FOR_AFTER_SWAP: Gas = Gas::from_tgas(80); // 70 were not enought
pub const GAS_FOR_WITHDRAW: Gas = Gas::from_tgas(55);
pub const GAS_FOR_ON_WITHDRAW_ON_SWAP: Gas = Gas::from_tgas(10);
pub const GAS_FOR_SWAP: Gas = Gas::from_tgas(20);

#[ext_contract(ext_ft_contract)]
trait ExtFtContract {
    fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>, msg: String) -> PromiseOrValue<U128>;
 }

#[derive(Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, Serialize))]
#[serde(crate = "near_sdk::serde")]
pub enum TokenReceiverMsg {
    Purchase {
        message: String,
        signature: String
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        let token_id = env::predecessor_account_id();
        assert_eq!(self.ft_account_id, token_id, "Wrong token");

        let amount = amount.0;

        let token_receiver_msg: TokenReceiverMsg = serde_json::from_str(&msg).expect("Can't parse TokenReceiverMsg");
        match token_receiver_msg {
            TokenReceiverMsg::Purchase { message, signature } => {
                self.nft_mint(message, signature, sender_id, amount);

                // events::emit::deposit(&sender_id, amount, &token_id);
                PromiseOrValue::Value(U128(0))
            }
        }
    }
}