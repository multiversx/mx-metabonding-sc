
ALICE="./interaction/wallets/alice.pem"
ADDRESS=$(erdpy data load --key=address-testnet)
DEPLOY_TRANSACTION=$(erdpy data load --key=deployTransaction-testnet)
PROXY=https://devnet-gateway.elrond.com
CHAINID=D
ESDT_ISSUE_COST=0xB1A2BC2EC50000 # 0.05 eGLD
ESDT_ISSUE_COST_DECIMAL=50000000000000000
ESDT_SYSTEM_SC_ADDRESS=erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u

ALICE_ADDRESS=0139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e1
SIGNER=0x2c5594ae2f77a913119bc9db52833245a5879674cd4aeaedcd92f6f9e7edf17d

deploy() {
    erdpy --verbose contract deploy --project=${PROJECT} \
    --arguments ${SIGNER} \
    --recall-nonce --pem=${ALICE} --gas-limit=100000000 --send --outfile="deploy.interaction.json" --proxy=${PROXY} --chain=${CHAINID} || return

    TRANSACTION=$(erdpy data parse --file="./deploy.interaction.json" --expression="data['emitted_tx']['hash']")
    ADDRESS=$(erdpy data parse --file="./deploy.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=address-testnet --value=${ADDRESS}
    erdpy data store --key=deployTransaction-testnet --value=${TRANSACTION}

    echo ""
    echo "Smart contract address: ${ADDRESS}"
}


addProject() {
    getContractAddressHex

    PROJECT=0x01
    PROJECT_OWNER=0x0139472eff6886771a982f3083da5d421f24c29181e63888228dc81ca60d69e1
    REWARD_TOKEN=0x4554482D343265663161
    REWARD_SUPPLY=0x3635C9ADC5DEA00000
    START_WEEK=0x1CB
    DURATION=0x64
    PERCENTAGE=0x32


    erdpy --verbose contract call ${CONTRACT_ADDRESS_HEX} --recall-nonce --pem=${ALICE} \
        --gas-limit=10000000 --function="addProject" \
        --arguments ${PROJECT} ${PROJECT_OWNER} ${REWARD_TOKEN} ${REWARD_SUPPLY} ${START_WEEK} ${DURATION} ${PERCENTAGE} \
        --send --proxy=${PROXY} --chain=${CHAINID} || return
}

fundProject() {
    getContractAddressHex
    
    erdpy --verbose contract call ${CONTRACT_ADDRESS_HEX} --recall-nonce --pem=${ALICE} \
        --gas-limit=5000000 --function="ESDTTransfer" \
        --arguments 0x4554482D343265663161 0x3635C9ADC5DEA00000 0x6465706F73697452657761726473 0x01 \
        --chain=${CHAINID} \
        --send --proxy=${PROXY} --chain=${CHAINID}
}

getContractAddress() {
    ADDRESS_BECH32=$(erdpy data parse --file="./deploy.interaction.json" --expression="data['emitted_tx']['address']")
    CONTRACT_ADDRESS=${ADDRESS_BECH32}
    echo "Contract address: ${CONTRACT_ADDRESS}"
}

getContractAddressHex() {
    getContractAddress
    CONTRACT_ADDRESS_HEX=$(erdpy wallet bech32 --decode $CONTRACT_ADDRESS)
}

issueWrappedEth() {
    local TOKEN_DISPLAY_NAME=0x57726170706564457468  # "WrappedEth"
    local TOKEN_TICKER=0x455448  # "ETH"
    local INITIAL_SUPPLY=0x3635C9ADC5DEA00000 # 1
    local NR_DECIMALS=0x12 # 18
    local CAN_ADD_SPECIAL_ROLES=0x63616e4164645370656369616c526f6c6573 # "canAddSpecialRoles"
    local TRUE=0x74727565 # "true"

    erdpy --verbose contract call ${ESDT_SYSTEM_SC_ADDRESS} --recall-nonce --pem=${ALICE} \
    --gas-limit=60000000 --value=${ESDT_ISSUE_COST_DECIMAL} --function="issue" \
    --arguments ${TOKEN_DISPLAY_NAME} ${TOKEN_TICKER} ${INITIAL_SUPPLY} ${NR_DECIMALS} ${CAN_ADD_SPECIAL_ROLES} ${TRUE} \
    --send --proxy=${PROXY} --chain=${CHAINID}
}
