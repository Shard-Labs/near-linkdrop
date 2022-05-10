# LinkDrop contract

LinkDrop contract allows any user to create a link that their friends can use to claim NFT tokens even if they don't have an account yet.

The way it works (remember this is an example, do not use the same account names):

Preliminar considerations:

- NFT contract is already created under account `nft_hodler`.

Deploy and initialize:

- Deploy the linkdrop smart contract with name `linkdrop`.
- Initialize the contract calling `linkdrop.init("nft_hodler")`.
- Transfer onwership of the NFT to `linkdrop`.

Sender, that has NEAR:

- Creates a new key pair `(pk1, privkey1)`.
- Calls `linkdrop.send(pk1)`.
- Sends a link to any supported wallet app with `privkey1` as part of URL.

Receiver, that doesn't have NEAR account:

- Receives link to the wallet with `privkey1`.
- Wallet creates new key pair for this user (or they generate it via HSM) `(pk2, privkey2)`.
- Enters the `new_account_id` receiver want for their new account.
- Wallet creates a transaction to `linkdrop.create_account_and_claim(new_account_id, pk2)` and singns it using `(pk1, privkey1)`.
- Contract creates new account with `new_account_id` name and `pk2` as full access key and transfers NFT ownership to `new_account_id`.

If Receiver already has account (or Sender wants to get back the money):

- Sign tx with `(pk1, privkey1)` to call `linkdrop.claim(account_id)`, which transfers the NFT ownership to `account_id`.
