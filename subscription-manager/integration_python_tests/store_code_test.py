import datetime
import pytest
from secret_sdk.key.mnemonic import MnemonicKey
from secret_sdk.core import Coins, TxResultCode
from secret_sdk.client.lcd.api.gov import ProposalStatus
from secret_sdk.core.wasm.msgs import MsgStoreCode, MsgInstantiateContract, MsgExecuteContract
from secret_sdk.protobuf import secret
from secret_sdk.util.tx import get_value_from_raw_log
from secret_sdk.protobuf.cosmos.tx.v1beta1 import BroadcastMode

@pytest.fixture
def mnemonics():
    # Initialize genesis accounts
    return [
        "grant rice replace explain federal release fix clever romance raise often wild taxi quarter soccer fiber love must tape steak together observe swap guitar",
        "jelly shadow frog dirt dragon use armed praise universe win jungle close inmate rain oil canvas beauty pioneer chef soccer icon dizzy thunder meadow",
        "chair love bleak wonder skirt permit say assist aunt credit roast size obtain minute throw sand usual age smart exact enough room shadow charge",
        "word twist toast cloth movie predict advance crumble escape whale sail such angry muffin balcony keen move employ cook valve hurt glimpse breeze brick",
    ]

def test_store_code():
    accounts = []

    for mnemonic in mnemonics:
        wallet = secret.wallet(MnemonicKey(mnemonic))
        accounts.append({
          'address': wallet.key.acc_address,
          'mnemonic': mnemonic,
          'wallet': wallet,
          'secret': secret
        })

    wallet = pytest.accounts[0]['wallet']

    with open(r'tests/data/snip20-ibc.wasm.gz', 'rb') as fl:
        wasm_byte_code = fl.read()

    msg_store_code = MsgStoreCode(
        sender=pytest.accounts[0]['address'],
        wasm_byte_code=wasm_byte_code,
        source='',
        builder=''
    )
    tx_store = wallet.create_and_broadcast_tx(
        [msg_store_code],
        gas='3000000',
        gas_prices=Coins('0.25uscrt')
    )
    if tx_store.code != TxResultCode.Success.value:
        raise Exception(f"Failed MsgStoreCode: {tx_store.raw_log}")
    assert tx_store.code == TxResultCode.Success.value

    code_id = int(get_value_from_raw_log(tx_store.rawlog, 'message.code_id'))

    code_info = pytest.secret.wasm.code_info(code_id)
    code_hash = code_info['code_info']['code_hash']
    pytest.sscrt_code_info = code_info
    pytest.sscrt_code_hash = code_hash

    msg_init = MsgInstantiateContract(
        sender=pytest.accounts[0]['address'],
        code_id=code_id,
        code_hash=code_hash,
        init_msg={
            "name": "Secret SCRT",
            "admin": pytest.accounts[0]['address'],
            "symbol": "SSCRT",
            "decimals": 6,
            "initial_balances": [{"address": pytest.accounts[0]['address'], "amount": "1"}],
            "prng_seed": "eW8=",
            "config": {
                "public_total_supply": True,
                "enable_deposit": True,
                "enable_redeem": True,
                "enable_mint": False,
                "enable_burn": False,
            },
            "supported_denoms": ["uscrt"],
        },
        label=f"Label {datetime.datetime.now()}",
        init_funds=[],
        encryption_utils=pytest.secret.encrypt_utils
    )
    tx_init = wallet.create_and_broadcast_tx(
        [msg_init],
        gas='5000000',
        gas_prices=Coins('0.25uscrt')
    )

    if tx_init.code != TxResultCode.Success.value:
        raise Exception(f"Failed MsgInstiateContract: {tx_init.raw_log}")
    assert tx_init.code == TxResultCode.Success.value
    assert get_value_from_raw_log(tx_init.rawlog, 'message.action') == "/secret.compute.v1beta1.MsgInstantiateContract"

    contract_adress = get_value_from_raw_log(tx_init.rawlog, 'message.contract_address')
    assert contract_adress == tx_init.data[0].address
    pytest.sscrt_contract_address = contract_adress

    msg_execute = MsgExecuteContract(
        sender=pytest.accounts[0]['address'],
        contract=contract_adress,
        msg={
            'create_viewing_key': {
                'entropy': 'bla bla'
            }
        },
        code_hash=code_hash,
        encryption_utils=pytest.secret.encrypt_utils
    )
    tx_execute = wallet.create_and_broadcast_tx(
        [msg_execute],
        gas='5000000',
        gas_prices=Coins('0.25uscrt')
    )
    if tx_execute.code != TxResultCode.Success.value:
        raise Exception(f"Failed MsgExecuteContract: {tx_execute.raw_log}")
    assert tx_execute.code == TxResultCode.Success.value
    assert '{"create_viewing_key":{"key":"' in tx_execute.data[0].data.decode('utf-8')

    tx = wallet.create_and_broadcast_tx(
        [msg_execute],
        gas='5000000',
        gas_prices=Coins('0.25uscrt'),
        broadcast_mode=BroadcastMode.BROADCAST_MODE_ASYNC
    )
    tx_hash = tx.txhash
    tx_execute = pytest.secret.tx.get_tx(tx_hash)
    while tx_execute is None:
        tx_execute = pytest.secret.tx.get_tx(tx_hash)
    if tx_execute.code != TxResultCode.Success.value:
        raise Exception(f"Failed MsgExecuteContract: {tx_execute.raw_log}")
    assert tx_execute.code == TxResultCode.Success.value
    assert '{"create_viewing_key":{"key":"' in tx_execute.data[0].data.decode('utf-8')
