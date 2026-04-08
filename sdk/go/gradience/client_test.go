package gradience

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func setupTestServer() (*httptest.Server, *Client) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		switch r.URL.Path {
		case "/api/wallets":
			if r.Method == "POST" {
				json.NewEncoder(w).Encode(map[string]string{"id": "w1", "name": "demo", "status": "active", "created_at": "2026-01-01T00:00:00Z"})
			} else {
				json.NewEncoder(w).Encode([]map[string]string{{"id": "w1", "name": "demo", "status": "active", "created_at": "2026-01-01T00:00:00Z"}})
			}
		case "/api/wallets/w1/balance":
			json.NewEncoder(w).Encode([]Balance{{ChainID: "eip155:8453", TokenAddress: "0x0", Balance: "1000", Decimals: 6}})
		case "/api/wallets/w1/fund":
			json.NewEncoder(w).Encode(map[string]string{"txHash": "0xabc"})
		case "/api/swap/quote":
			json.NewEncoder(w).Encode(map[string]string{"from_token": "0xA", "to_token": "0xB", "from_amount": "1000", "to_amount": "2000", "chain": "base"})
		case "/api/ai/generate":
			json.NewEncoder(w).Encode(map[string]string{"text": "hello", "cost": "0.001"})
		default:
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(map[string]string{"error": "not found"})
		}
	}))

	client := NewClient(server.URL, "test-token")
	return server, client
}

func TestCreateWallet(t *testing.T) {
	server, client := setupTestServer()
	defer server.Close()

	wallet, err := client.CreateWallet("demo")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if wallet.ID != "w1" {
		t.Errorf("expected id w1, got %s", wallet.ID)
	}
}

func TestListWallets(t *testing.T) {
	server, client := setupTestServer()
	defer server.Close()

	wallets, err := client.ListWallets()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(wallets) != 1 {
		t.Errorf("expected 1 wallet, got %d", len(wallets))
	}
}

func TestGetBalance(t *testing.T) {
	server, client := setupTestServer()
	defer server.Close()

	balances, err := client.GetBalance("w1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if balances[0].Balance != "1000" {
		t.Errorf("expected balance 1000, got %s", balances[0].Balance)
	}
}

func TestFundWallet(t *testing.T) {
	server, client := setupTestServer()
	defer server.Close()

	res, err := client.FundWallet("w1", "0xto", "1", "base")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if res["txHash"] != "0xabc" {
		t.Errorf("unexpected txHash: %v", res["txHash"])
	}
}

func TestSwapQuote(t *testing.T) {
	server, client := setupTestServer()
	defer server.Close()

	res, err := client.SwapQuote("w1", map[string]string{
		"from_token": "0xA",
		"to_token":   "0xB",
		"amount":     "1000",
		"chain":      "base",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if res["to_amount"] != "2000" {
		t.Errorf("unexpected to_amount: %v", res["to_amount"])
	}
}

func TestAIGenerate(t *testing.T) {
	server, client := setupTestServer()
	defer server.Close()

	res, err := client.AIGenerate("w1", "claude", "hi")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if res["text"] != "hello" {
		t.Errorf("unexpected text: %v", res["text"])
	}
}
