NEAR Speed NFT Trading
======

Demo: `n1.pepeproject.testnet`

----
Common Arguments
===
`account_id` - Near Account

`token_id` - String, "`<ipfs_hash>`" OR "`<generation>:<ipfs_hash>`", ipfs_hash contains the NFT media.

`generation` - number of sales for a given `token_id`

Server Parameters
======

`get_ft_account_id` - whitelsited FT contract

`get_public_key` - get Publick Key for signatures

Market
===

`get_token(token_id)` - returns [token, [generation, price, last_sale_timestamp]]

`get_token_for_sale(token_id)` - returns [token, next_price]

`ft_transfer_call` - purchase NFT. Minting NFT, sending FT to the seller, paying referral commission, saving NFT in the seller's collection (if required and if possible).
The token for sale has a token_id equals to "`<ipfs_hash>`". The token in the collection has a `token_id` equals to "`<generation>:<ipfs_hash>`".

Example: https://testnet.nearblocks.io/txns/2aHrHL2MDU9NdSbFBJ4QBmSVE5Tv7V92t9rpueorGsSR#execution

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

NFT Interface
===

`nft_token(token_id)` token_id is "`<ipfs_hash>`" OR "`<generation>:<ipfs_hash>`"

`nft_total_supply`

`nft_tokens(from_index, limit)`

`nft_supply_for_owner(account_id)`

`nft_tokens_for_owner(account_id, from_index, limit)`
