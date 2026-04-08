# Gradience Ruby SDK

Ruby SDK for the Gradience Wallet API.

## Install

```bash
gem build gradience.gemspec
gem install ./gradience-0.1.0.gem
```

Or add to your `Gemfile`:

```ruby
gem 'gradience', path: './sdk/ruby'
```

## Usage

```ruby
require 'gradience'

client = Gradience::Client.new('http://localhost:8080', api_token: 'YOUR_TOKEN')

wallet = client.create_wallet('demo')
puts wallet['id']

balance = client.get_balance(wallet['id'])
puts balance

quote = client.swap_quote(
  wallet['id'],
  from_token: '0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913',
  to_token: '0x4200000000000000000000000000000000000006',
  amount: '1000000',
  chain: 'base'
)
puts quote['to_amount']
```

## API Coverage

- `create_wallet(name)`
- `list_wallets`
- `get_balance(wallet_id)`
- `fund_wallet(wallet_id, to:, amount:, chain:)`
- `swap_quote(wallet_id, from_token:, to_token:, amount:, chain:)`
- `ai_generate(wallet_id, model:, prompt:)`

See [`docs/06-sdk-guide.md`](../../docs/06-sdk-guide.md) for the full SDK roadmap.

## License

MIT
