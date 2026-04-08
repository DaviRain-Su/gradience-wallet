# Gradience Python SDK

Python SDK for interacting with the Gradience Wallet API.

## Install

```bash
pip install -e .
```

## Usage

```python
from gradience_sdk import GradienceClient

client = GradienceClient(base_url="https://api.gradience.example.com", api_token="your-token")

# Create a wallet
wallet = client.create_wallet("my-wallet")
print(wallet["id"])

# Get balances
balances = client.get_balance(wallet["id"])
for b in balances:
    print(b["chain_id"], b["balance"])

# Fund wallet
result = client.fund_wallet(wallet["id"], to="0x...", amount="0.001", chain="base")
print(result["tx_hash"])

# Sign transaction
signed = client.sign_transaction(wallet["id"], {
    "to": "0x...",
    "value": "1000",
    "data": "0x",
    "chainId": "8453"
})
print(signed["signed_tx"])

# DEX swap quote
quote = client.swap_quote(wallet["id"], {
    "from_token": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    "to_token": "0x4200000000000000000000000000000000000006",
    "amount": "1000000",
    "chain": "base",
})
print(quote["to_amount"])

# AI generation
ai = client.ai_generate(wallet["id"], "claude-3-5-sonnet", "Summarize DeFi trends")
print(ai["text"])
```

See [`docs/06-sdk-guide.md`](../../docs/06-sdk-guide.md) for the full SDK roadmap.

## License

MIT
