#!/bin/bash

LINKDROP = "res/linkdrop.wasm"
DEPOSIT=4000000000000000000000000
GAS=300000000000000
master_account=$2

case $1 in
    '--create-sub-account')
    # Create sub-account for Linkdrop contract deployment
        echo 'Create sub account...'
        near create-account linkdrop."$master_account" --masterAccount "$master_account"
        echo'Sub account created: ' linkdrop."$master_account"
        ;;
    
    '--deploy-factory')
        echo 'Deploying Linkdrop...'
        near deploy --accountId linkdrop."$master_account" --wasmFile $LINKDROP
        echo '"Linkdrop deployed.'
        args=$3
        if [ -z "$args" ]; then
            args='{"nft_contract_id": "dummy-nft.dexterdev8.testnet"}'
        fi
        echo $args
        # Initialize Linkdrop contract
        near call linkdrop."$master_account" init "$args" \
            --accountId "$master_account"
        echo 'Linkdrop initialized.'
        ;;
    
    '--send')
        # Contract sets given public key access to call claim functionalities
        echo 'Send PKs'
        args=$3
        if [ -z "$args" ]; then
            args='{"public_keys": ["J1Q32xAYCiDmwwcf7A3c4XGsPgN5AyvioVZJfDHpoaeN", "9xhcBQka1gKhS8rufWUC4TqAikF5nVucWVbW8vuKFxEx"]}'
        fi
        near call linkdrop."$master_account" send "$args" \
            --accountId "$master_account" \
            --depositYocto $DEPOSIT \
            --gas $GAS
        ;;

    '--claim')
        # Claim reward for given account id
        echo 'Claim reward'
        node tx_claim.js
        ;;

    '--create-acc-claim')
        # Creates a subaccount under linkdrop."$master_account" and claims the reward for the new account
        echo 'Create account and claim reward'
        node tx_create_account_and_claim.js
        ;;
    
    '--public_key_is_claimable')
        # Checks if given public key is eligible to claim a reward within this contract
        echo 'Check if public key is claimable'
        args=$3
        if [ -z "$args" ]; then
            args='{"public_key": "J1Q32xAYCiDmwwcf7A3c4XGsPgN5AyvioVZJfDHpoaeN"}'
        fi
        near view linkdrop."$master_account" public_key_is_claimable \
            "$args"
        ;;

