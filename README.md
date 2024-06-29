NEAR Speed NFT Trading
======

Demo: `n1.pepeproject.testnet`

----
Common Arguments
===
`account_id` - Near Account

`token_id` - String, "`<ipfs_hash>`" OR "`<generation>:<ipfs_hash>`", ipfs_hash contains the NFT media.

`generation` - number of sales for a given `token_id`

Global Parameters
======

`get_ft_account_id` - whitelsited FT contract

`get_public_key` - get Public Key for signatures

Market
===

`get_token(token_id)` - returns [token, [generation, price]]

`get_token_for_sale(token_id)` - returns [token, next_price, seller_collection_items, seller_is_store_tokens]
seller_collection_items - u64, seller_is_store_tokens - bool

`ft_transfer_call` - NFT actions 

### NFT Purchase 
Minting NFT, sending FT to the seller, paying referral commission, saving NFT in the seller's collection (if required and if possible).
The token for sale has a token_id equals to "`<ipfs_hash>`". The token in the collection has a `token_id` equals to "`<generation>:<ipfs_hash>`".

````
Example:
{
"receiver_id": "n1.pepeproject.testnet",
"amount": "100",
"msg": "{\"Purchase\":{\"message\":\"{\\\"SimpleMint\\\":{\\\"token_id\\\":\\\"9.jpg\\\",\\\"account_id\\\":\\\"pepeproject.testnet\\\",\\\"referral_id_1\\\":\\\"zavodil2.testnet\\\",\\\"referral_id_2\\\":null,\\\"timestamp\\\":1710796871868251000}}\",\"signature\":\"208c14a1b64479dc4a5496ede8331f0f58f73e91db268f27bed592b4c05b08cd1c006ac49eaf0e5caf1786b108a6907b62e11a9f20e5b11cbab92533f898030e\"}}"
}
````

msg parameter:
```
Purchase: {
    message:
        SimpleMint: {
           "token_id": "<ipfs_hash>",
           "account_id": "buyer_name.near",           
           "referral_id_1": "ref1.near",
           "referral_id_2": "ref2.near",
           "timestamp": 123123123,
        },
    signature
}
```
`signature` - message signed with self.public_key

**This function doesn't check if buyer has enough storage to keep the token. We expect server to make this check before to verify the transaction.**

Example: https://testnet.nearblocks.io/txns/2aHrHL2MDU9NdSbFBJ4QBmSVE5Tv7V92t9rpueorGsSR#execution

### Request to add NFT

msg parameter:
```
Purchase: {
    message:
        UserMintRequest {
           "token_id": "<ipfs_hash>",
           "account_id": "nft_owner_name.near",           
        },
    signature
}
```
Requires self.user_mint_price FT to be attached

### Admin response to add NFT

msg parameter:
```
Purchase: {
    message:
        UserMintResponses {
           "responses": [
                {
                    "user_token_request_id": u64,
                    "token_id: "<ipfs_hash>",,
                    "mint_price": U128
                    "token_status": "APPROVED"/"REEJCTED",
                }
           ]           
        },
    signature
}
```
Requires 1 FT to be attached (just 1, not 10^decimals)

User Balance
======

`get_balance(account_id)` - read user's virtual balance

`withdraw(amount)` - withdraw virtual balance (referral fees, failed withdrawals)

User Collection
===

**Internal storage of unsellable NFTs**

`get_collection(account_id)` - array of NFTs stored by user

`get_collection_items(account_id)` - quantity of NFTs stored by user

`set_store_user_tokens(account_id)` - set a parameter to determine whether the user wants to attempt to save copies of NFTs to their collection upon resale

`get_store_user_tokens` - get the parameter above

`remove_user_collection_item(generation, token_id)` - remove NFT from collection

Prepaid Storage
====

`get_storage_packages` -> Vec[index, storage_size, price]

`get_user_storage(account_id)` -> storage_size

`get_free_storage_size` -> storage_size (3)

`get_max_storage_size` -> storage_size (25)

To buy storage, execute `ft_transfer_call`, example:

```
near call 438e48ed4ce6beecf503d43b9dbd3c30d516e7fd.factory.bridge.near ft_transfer_call '{"receiver_id": "'$CONTRACT_ID'", "amount": "1000000000000000000", "msg": "{\"Storage\":{\"index\":1}}"}' --accountId $OWNER_ID --depositYocto 1 --gas 50000000000000
```
Don't forget to set a proper `index` and attach corresponding amount of FT

`transfer_storage (receiver_contract_id: AccountId)` -> delete storage from current contract and add it on receiver_contract_id 

NFT Interface
===

`nft_token(token_id)` token_id is "`<ipfs_hash>`" OR "`<generation>:<ipfs_hash>`"

`nft_total_supply`

`nft_tokens(from_index, limit)`

`nft_supply_for_owner(account_id)`

`nft_tokens_for_owner(account_id, from_index, limit)`

User tokens
===

`get_user_mint_requests(from_index, limit)` - read user requests. Returns 
[[user_token_request_id, UserToken], [user_token_request_id, UserToken], ..] 
Where `UserToken` is
```
{
    pub token_id: <ipfs_hash>,
    pub account_id: AccountId,
    pub status: PENDING/APPROVED"/"REEJCTED,
    pub created_at: Timestamp
}
```