use crate::*;

pub mod emit {
    use super::*;
    use near_sdk::serde_json::json;

    #[derive(Serialize)]
    #[serde(crate = "near_sdk::serde")]
    struct AccountAmountData<'a> {
        pub account_id: &'a AccountId,
        #[serde(with = "u128_dec_format")]
        pub amount: Balance,
    }

    #[derive(Serialize)]
    #[serde(crate = "near_sdk::serde")]
    struct AccountTokenAmountData<'a> {
        pub account_id: &'a AccountId,
        pub token_id: &'a TokenId,
        #[serde(with = "u128_dec_format")]
        pub amount: Balance,
    }

    #[derive(Serialize)]
    #[serde(crate = "near_sdk::serde")]
    struct ReferralTokenAmountData<'a> {
        pub referrer_id: &'a AccountId,
        pub account_id: &'a AccountId,
        pub token_id: &'a TokenId,
        #[serde(with = "u128_dec_format")]
        pub amount: Balance,
    }

    fn log_event<T: Serialize>(event: &str, data: T) {
        let event = json!({
            "standard": "nftinder",
            "version": "1.0.0",
            "event": event,
            "data": [data]
        });

        log!("EVENT_JSON:{}", event.to_string());
    }

    pub fn add_referral_fee(referrer_id: &AccountId, account_id: &AccountId, token_id: &TokenId, amount: Balance) {
        log_event("referral_fee", ReferralTokenAmountData { referrer_id, account_id, token_id, amount });
    }

    pub fn add_system_fee(account_id: &AccountId, token_id: &TokenId, amount: Balance) {
        log_event("system_fee", AccountTokenAmountData { account_id, token_id, amount });
    }

    pub fn add_seller_payout(account_id: &AccountId, token_id: &TokenId, amount: Balance) {
        log_event("seller_payout", AccountTokenAmountData { account_id, token_id, amount });
    }

    pub fn add_withdraw(account_id: &AccountId, amount: Balance) {
        log_event("withdraw", AccountAmountData { account_id, amount });
    }
}

pub mod u128_dec_format {
    use near_sdk::serde::Serializer;

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }
}
