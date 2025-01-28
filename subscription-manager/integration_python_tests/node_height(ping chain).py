import os
from dotenv import load_dotenv
from secret_sdk.client.lcd import LCDClient

# Загрузите переменные из .env файла
load_dotenv()

# Получите значения из переменных окружения
chain_id = os.getenv('CHAIN_ID')
node_url = os.getenv('NODE_URL')

# Используйте значения для создания LCDClient
secret = LCDClient(chain_id=chain_id, url=node_url)
height = secret.tendermint.block_info()['block']['header']['height']
print(height)
