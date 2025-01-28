import os
from time import sleep

from dotenv import load_dotenv
from secret_sdk.client.lcd import LCDClient
from secret_sdk.client.localsecret import LocalSecret, main_net_chain_id, test_net_chain_id
from secret_sdk.core.coins import Coins
from secret_sdk.key.mnemonic import MnemonicKey
from secret_sdk.protobuf.cosmos.tx.v1beta1 import BroadcastMode

load_dotenv()
chain_id = os.getenv('CHAIN_ID')
contract = os.getenv('CONTRACT_ADDRESS')
node_url = os.getenv('NODE_URL')
mnemonic = os.getenv('MNEMONIC')

print("chain_id: " + chain_id)
print("node_url: " + node_url)
print("contract: " + contract)
print("mnemonic: " + mnemonic)

mk = MnemonicKey(mnemonic=mnemonic)
secret = LCDClient(chain_id=chain_id, url=node_url)
wallet = secret.wallet(mk)

wallet_public_key = str(wallet.key.acc_address)

print("wallet_public_key: " + wallet_public_key)

contract_address = contract
sent_funds = Coins('100uscrt')

public_key = "secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03"
handle_msg = {"set_admin": {"public_key": public_key}}

t = wallet.execute_tx(
    contract_addr=contract_address,
    handle_msg=handle_msg,
    transfer_amount=sent_funds,
)

print(t)

assert t.code == 0, f"Transaction failed with code {t.code}: {t.rawlog}"
print("Transaction successful:", t.txhash)

sleep(5)

tx_info = secret.tx.tx_info(
    tx_hash=t.txhash,
)
print("Transaction info:", tx_info)
