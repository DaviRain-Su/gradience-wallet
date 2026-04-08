# Gradience Go SDK

Go SDK for the Gradience Wallet API.

## Install

```bash
go get github.com/gradience-wallet/sdk/go/gradience
```

## Usage

```go
package main

import (
	"fmt"
	"log"

	"github.com/gradience-wallet/sdk/go/gradience"
)

func main() {
	client := gradience.NewClient("http://localhost:8080", "YOUR_TOKEN")

	wallet, err := client.CreateWallet("demo")
	if err != nil {
		log.Fatal(err)
	}
	fmt.Println("Wallet ID:", wallet.ID)

	balances, err := client.GetBalance(wallet.ID)
	if err != nil {
		log.Fatal(err)
	}
	for _, b := range balances {
		fmt.Println(b.ChainID, b.Balance)
	}
}
```

## API Coverage

- `CreateWallet(name string)`
- `ListWallets()`
- `GetBalance(walletID string)`
- `FundWallet(walletID, to, amount, chain string)`
- `SwapQuote(walletID string, params map[string]string)`
- `AIGenerate(walletID, model, prompt string)`

See [`docs/06-sdk-guide.md`](../../docs/06-sdk-guide.md) for the full SDK roadmap.

## License

MIT
