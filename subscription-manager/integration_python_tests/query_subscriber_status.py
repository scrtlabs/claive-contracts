import os

from dotenv import load_dotenv
from secret_sdk.client.lcd import LCDClient
from secret_sdk.client.localsecret import LocalSecret, main_net_chain_id, test_net_chain_id
from secret_sdk.core.coins import Coins
from secret_sdk.key.mnemonic import MnemonicKey

load_dotenv()
chain_id = os.getenv('CHAIN_ID')
contract = os.getenv('CONTRACT_ADDRESS')
node_url = os.getenv('NODE_URL')

print("chain_id: " + chain_id)
print("node_url: " + node_url)
print("contract: " + contract)

secret = LCDClient(chain_id=chain_id, url=node_url)

public_key = "subscriber"

query = {"subscriber_status":{"public_key":public_key}}

result = secret.wasm.contract_query(contract, query)

print(result)
