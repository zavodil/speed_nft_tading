use crate::*;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::json_types::U128;
use near_sdk::{serde_json, Promise, PromiseOrValue, PromiseResult};

pub const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(10);
pub const GAS_FOR_AFTER_FT_TRANSFER: Gas = Gas::from_tgas(10);

#[ext_contract(ext_ft_contract)]
trait ExtFtContract {
    fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>, msg: String) -> PromiseOrValue<U128>;
 }

#[ext_contract(ext_self)]
pub trait ExtContract {
    fn callback_after_withdraw(&mut self, sender_id: AccountId, amount: U128);
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

#[near_bindgen]
impl Contract {
    #[private]
    pub fn callback_after_withdraw(&mut self, sender_id: AccountId, amount: U128) {
        assert_eq!(env::promise_results_count(), 1, "Err: expected 1 promise result from withdraw");
        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                // TODO
                //events::emit::unstake_succeeded(&sender_id, amount.0, &token_id);
            }
            PromiseResult::Failed => {
                self.internal_add_balance(&sender_id, amount.0);

                // TODO
                // events::emit::unstake_failed(&sender_id, amount.0, &token_id);
            }
        };
    }

}

impl Contract {
    // send tokens on withdraw
    pub fn internal_ft_transfer(&mut self, account_id: &AccountId, amount: Balance) -> Promise {
        ext_ft_core::ext(self.ft_account_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .ft_transfer(account_id.clone(), amount.into(), None)
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_AFTER_FT_TRANSFER)
                    .callback_after_withdraw(account_id.clone(), amount.into()),
            )
    }
}