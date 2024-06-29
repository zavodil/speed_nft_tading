use crate::*;

pub mod emit {
    use super::*;
    use near_sdk::serde_json::json;

    #[derive(Serialize)]
    #[serde(crate = "near_sdk::serde")]
    struct AccountTokenData<'a> {
        pub account_id: &'a AccountId,
        pub token_id: &'a TokenId,
    }

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
        // referrer_id => authorized_id
        pub authorized_id: &'a AccountId,
        pub account_id: &'a AccountId,
        pub token_id: &'a TokenId,
        #[serde(with = "u128_dec_format")]
        pub amount: Balance,
    }

    #[derive(Serialize)]
    #[serde(crate = "near_sdk::serde")]
    struct UserTokenRequestData<'a> {
        pub account_id: &'a AccountId,
        pub token_id: &'a TokenId,
        pub status: &'a TokenStatus,
    }

    #[derive(Serialize)]
    #[serde(crate = "near_sdk::serde")]
    struct AccountStorageData<'a> {
        pub account_id: &'a AccountId,
        pub storage: StorageSize,
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

    pub fn token_status_changed(account_id: &AccountId, token_id: &TokenId, status: &TokenStatus) {
        log_event("token_status_changed", UserTokenRequestData { account_id, token_id, status });
    }

    pub fn add_referral_fee(referrer_id: &AccountId, account_id: &AccountId, token_id: &TokenId, amount: Balance) {
        log_event("referral_fee", ReferralTokenAmountData { authorized_id: referrer_id, account_id, token_id, amount });
    }

    pub fn add_system_fee(account_id: &AccountId, token_id: &TokenId, amount: Balance) {
        log_event("system_fee", AccountTokenAmountData { account_id, token_id, amount });
    }

    pub fn add_seller_payout(account_id: &AccountId, token_id: &TokenId, amount: Balance) {
        log_event("seller_payout", AccountTokenAmountData { account_id, token_id, amount });
    }

    pub fn add_deposit(account_id: &AccountId, amount: Balance) {
        log_event("deposit", AccountAmountData { account_id, amount });
    }

    pub fn add_storage(account_id: &AccountId, amount: Balance) {
        log_event("storage", AccountAmountData { account_id, amount });
    }

    pub fn storage_transferred(account_id: &AccountId, storage: StorageSize) {
        log_event("storage_transferred", AccountStorageData { account_id, storage });
    }
    pub fn transfer_storage_succeeded(account_id: &AccountId, storage: StorageSize) {
        log_event("transfer_storage_succeeded", AccountStorageData { account_id, storage });
    }

    pub fn transfer_storage_failed(account_id: &AccountId, storage: StorageSize) {
        log_event("transfer_storage_failed", AccountStorageData { account_id, storage });
    }

    pub fn add_withdraw_succeeded(account_id: &AccountId, amount: Balance) {
        log_event("withdraw_succeeded", AccountAmountData { account_id, amount });
    }

    pub fn add_withdraw_failed(account_id: &AccountId, amount: Balance) {
        log_event("withdraw_failed", AccountAmountData { account_id, amount });
    }

    pub fn add_burn_nft(account_id: &AccountId, token_id: &TokenId) {
        log_event("nft_burn", AccountTokenData { account_id, token_id });
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
