use std::ops::Deref;
use crate::*;
use near_sdk::{log, NearToken, Promise};
use near_sdk::env::predecessor_account_id;

#[near_bindgen]
impl Contract {
    pub fn get_ft_account_id(&self) -> AccountId {
        self.ft_account_id.clone()
    }

    pub fn get_collection_items(&self, account_id: AccountId) -> StorageSize {
        self.get_user_collection_items(&account_id)
    }

    pub fn set_store_user_tokens(&mut self, store_tokens: bool) {
        self.is_store_user_tokens.insert(env::predecessor_account_id(), store_tokens);
    }

    pub fn get_store_user_tokens(&self, account_id: AccountId) -> bool {
        self.is_store_user_tokens.get(&account_id).unwrap_or(&false).clone()
    }

    pub fn set_min_mint_price(&mut self, min_mint_price: U128) {
        self.assert_owner();
        self.min_mint_price = min_mint_price.0;
    }

    pub fn get_public_key(&mut self) -> String {
        self.public_key.clone()
    }

    pub fn set_public_key(&mut self, public_key: String) {
        self.assert_owner();
        self.public_key = public_key;
    }

    pub fn set_mint_price_increase_fee(&mut self, mint_price_increase_fee: FeeFraction) {
        self.assert_owner();
        self.mint_price_increase_fee = mint_price_increase_fee;
    }

    pub fn set_seller_fee(&mut self, seller_fee: FeeFraction) {
        self.assert_owner();
        self.seller_fee = seller_fee;
    }

    pub fn set_referral_fee(&mut self, referral_1_fee: FeeFraction, referral_2_fee: FeeFraction) {
        self.assert_owner();
        self.referral_1_fee = referral_1_fee;
        self.referral_2_fee = referral_2_fee;
    }

    pub fn get_balance(&self, account_id: AccountId) -> U128 {
        U128::from(self.internal_balances.get(&account_id).unwrap_or(&0u128).clone())
    }

    pub fn remove_user_collection_item(&mut self, generation: TokenGeneration, token_id: TokenId) {
        let account_id = predecessor_account_id();

        let mut user_collection = self.user_collection_items.get(&account_id).expect("Not found");

        let item_to_remove = CollectionItem {token_id: token_id.clone(), generation};
        if user_collection.contains(&item_to_remove) {
            log!("Item removed: {}:{}", generation, token_id.clone());

            user_collection.remove(&item_to_remove);
            self.user_collection_items.insert(&account_id, &user_collection);
        }
        else {
            panic!("Not found");
        }

        // remove NFT
        let full_token_id = generate_token_id(&generation, &token_id);
        self.tokens.owner_by_id.remove(&full_token_id);

        if let Some(tokens_per_owner) = &mut self.tokens.tokens_per_owner {
            let mut token_ids = tokens_per_owner.get(&account_id).expect("Not found");
            token_ids.remove(&token_id);
            tokens_per_owner.insert(&account_id, &token_ids);
        }
    }

    pub fn claim(&mut self, amount: Option<U128>) -> Promise {
        let account_id = env::predecessor_account_id();
        let balance: Balance = self.internal_balances.get(&account_id).unwrap_or(&0u128).clone();

        let amount: Balance = if let Some(amount) = amount {
            assert!(balance >= amount.0, "Balance is too small");
            amount.0
        } else {
            balance
        };

        self.internal_balances
            .insert(account_id.clone(), balance - amount);

        Promise::new(account_id).transfer(NearToken::from_yoctonear(amount))
    }

    // returns [token, [generation, price, last_sale]]
    pub fn get_token(&self, token_id: TokenId) -> (Option<Token>, Option<(TokenGeneration, U128, Option<Timestamp>)>) {
        if let Some(token) = self.tokens.nft_token(token_id.clone()) {
            let token_data = self.get_token_data(&token_id);
            (
                Some(token),
                Some((token_data.generation, U128::from(token_data.price), token_data.last_sale))
            )
        } else {
            (None, None)
        }
    }

    // returns [token, next_price]
    pub fn get_token_for_sale(&self, token_id: TokenId) -> Option<(Token, U128)> {
        if let Some(token) = self.tokens.nft_token(token_id.clone()) {
            let old_price: Balance = self.get_token_price(&token_id);

            let price_increase = self.mint_price_increase_fee.multiply(old_price);

            Some((token, U128::from(old_price + price_increase)))
        } else {
            None
        }
    }
}

impl Contract {
    pub fn assert_owner(&self) {
        assert_eq!(self.owner_id, env::predecessor_account_id(), "Not an owner");
    }

    pub(crate) fn internal_add_balance(&mut self, account_id: &AccountId, value: Balance) {
        if value > 0 {
            let prev_balance: Balance = self.internal_balances.get(account_id).unwrap_or(&0u128).clone();
            self.internal_balances.insert(account_id.clone(), prev_balance + value);

            // TODO TODO emit event new reward
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct FeeFraction {
    pub numerator: u32,
    pub denominator: u32,
}

impl FeeFraction {
    pub fn assert_valid(&self) {
        assert_ne!(self.denominator, 0, "Denominator must be a positive number");
        assert!(
            self.numerator <= self.denominator,
            "The treasure fee must be less or equal to 1"
        );
    }

    pub fn multiply(&self, value: Balance) -> Balance {
        (U256::from(self.numerator) * U256::from(value) / U256::from(self.denominator)).as_u128()
    }
}

use uint::construct_uint;
construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}
