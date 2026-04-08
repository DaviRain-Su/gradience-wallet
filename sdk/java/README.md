# Gradience Java SDK

Java SDK for the Gradience Wallet API.

## Install

Add to your `build.gradle`:

```gradle
implementation 'io.gradience:sdk:0.1.0'
```

Or clone and build locally:

```bash
cd sdk/java
./gradlew build
```

## Usage

```java
import io.gradience.sdk.GradienceClient;
import io.gradience.sdk.Wallet;
import io.gradience.sdk.Balance;
import io.gradience.sdk.GradienceException;

public class Main {
    public static void main(String[] args) {
        GradienceClient client = new GradienceClient("http://localhost:8080", "YOUR_TOKEN");
        try {
            Wallet wallet = client.createWallet("demo");
            System.out.println("Wallet ID: " + wallet.id);

            for (Balance b : client.getBalance(wallet.id)) {
                System.out.println(b.chain_id + " " + b.balance);
            }
        } catch (GradienceException e) {
            System.err.println(e.getMessage());
        }
    }
}
```

## API Coverage

- `createWallet(String name)`
- `listWallets()`
- `getBalance(String walletId)`
- `fundWallet(String walletId, String to, String amount, String chain)`
- `swapQuote(String walletId, String fromToken, String toToken, String amount, String chain)`
- `aiGenerate(String walletId, String model, String prompt)`

See [`docs/06-sdk-guide.md`](../../docs/06-sdk-guide.md) for the full SDK roadmap.

## License

MIT
