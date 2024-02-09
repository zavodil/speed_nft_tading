use crate::*;
use near_sdk::{NearToken, Promise};

#[near_bindgen]
impl Contract {
    pub fn set_min_mint_price(&mut self, min_mint_price: U128) {
        self.assert_owner();
        self.min_mint_price = min_mint_price.0;
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

    // returns [internal_balance, referral_id]
    pub fn get_account(&self, account_id: AccountId) -> (U128, Option<AccountId>) {
        (
            U128::from(self.internal_balances.get(&account_id).unwrap_or_default()),
            self.referrals.get(&account_id),
        )
    }

    pub fn claim(&mut self, amount: Option<U128>) -> Promise {
        let account_id = env::predecessor_account_id();
        let balance = self.internal_balances.get(&account_id).unwrap_or_default();

        let amount: Balance = if let Some(amount) = amount {
            assert!(balance >= amount.0, "Balance is too small");
            amount.0
        } else {
            balance
        };

        self.internal_balances
            .insert(&account_id, &(balance - amount));
        Promise::new(account_id).transfer(NearToken::from_yoctonear(amount))
    }

    // returns [token, next_price]
    pub fn get_token_for_sale(&self, token_id: TokenId) -> Option<(Token, U128)> {
        let old_price: Balance = self
            .token_prices
            .get(&token_id)
            .expect("Token price is missing");
        let price_increase = self.mint_price_increase_fee.multiply(old_price);

        if let Some(token) = self.tokens.nft_token(token_id) {
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
            let prev_balance = self.internal_balances.get(account_id).unwrap_or_default();
            self.internal_balances
                .insert(&account_id, &(prev_balance + value));

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
