package gradience

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"time"
)

// GradienceError represents an API error.
type GradienceError struct {
	Message    string
	StatusCode int
	Body       interface{}
}

func (e *GradienceError) Error() string {
	return e.Message
}

// Client wraps the Gradience REST API.
type Client struct {
	BaseURL   string
	APIToken  string
	HTTPClient *http.Client
}

// NewClient creates a new Gradience client.
func NewClient(baseURL, apiToken string) *Client {
	return &Client{
		BaseURL:    baseURL,
		APIToken:   apiToken,
		HTTPClient: &http.Client{Timeout: 30 * time.Second},
	}
}

func (c *Client) headers() http.Header {
	h := http.Header{}
	h.Set("Content-Type", "application/json")
	if c.APIToken != "" {
		h.Set("Authorization", "Bearer "+c.APIToken)
	}
	return h
}

func (c *Client) request(method, path string, body interface{}, query url.Values) (interface{}, error) {
	u, err := url.Parse(c.BaseURL + path)
	if err != nil {
		return nil, err
	}
	if query != nil {
		u.RawQuery = query.Encode()
	}

	var bodyReader io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return nil, err
		}
		bodyReader = bytes.NewReader(b)
	}

	req, err := http.NewRequest(method, u.String(), bodyReader)
	if err != nil {
		return nil, err
	}
	req.Header = c.headers()

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, &GradienceError{Message: err.Error()}
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	var data interface{}
	_ = json.Unmarshal(respBody, &data)
	if data == nil {
		data = string(respBody)
	}

	if resp.StatusCode >= 400 {
		msg := fmt.Sprintf("%v", data)
		if m, ok := data.(map[string]interface{}); ok {
			if e, ok := m["error"].(string); ok {
				msg = e
			}
		}
		return nil, &GradienceError{Message: msg, StatusCode: resp.StatusCode, Body: data}
	}

	return data, nil
}

// Wallet represents a Gradience wallet.
type Wallet struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Status    string `json:"status"`
	CreatedAt string `json:"created_at"`
}

// CreateWallet creates a new wallet.
func (c *Client) CreateWallet(name string) (*Wallet, error) {
	res, err := c.request("POST", "/api/wallets", map[string]string{"name": name}, nil)
	if err != nil {
		return nil, err
	}
	var w Wallet
	b, _ := json.Marshal(res)
	_ = json.Unmarshal(b, &w)
	return &w, nil
}

// ListWallets lists all wallets.
func (c *Client) ListWallets() ([]Wallet, error) {
	res, err := c.request("GET", "/api/wallets", nil, nil)
	if err != nil {
		return nil, err
	}
	var wallets []Wallet
	b, _ := json.Marshal(res)
	_ = json.Unmarshal(b, &wallets)
	return wallets, nil
}

// Balance represents a token balance.
type Balance struct {
	ChainID      string `json:"chain_id"`
	TokenAddress string `json:"token_address"`
	Balance      string `json:"balance"`
	Decimals     int    `json:"decimals"`
}

// GetBalance returns balances for a wallet.
func (c *Client) GetBalance(walletID string) ([]Balance, error) {
	res, err := c.request("GET", "/api/wallets/"+walletID+"/balance", nil, nil)
	if err != nil {
		return nil, err
	}
	var balances []Balance
	b, _ := json.Marshal(res)
	_ = json.Unmarshal(b, &balances)
	return balances, nil
}

// FundWallet funds a wallet.
func (c *Client) FundWallet(walletID, to, amount, chain string) (map[string]interface{}, error) {
	res, err := c.request("POST", "/api/wallets/"+walletID+"/fund", map[string]string{
		"to":     to,
		"amount": amount,
		"chain":  chain,
	}, nil)
	if err != nil {
		return nil, err
	}
	m, _ := res.(map[string]interface{})
	return m, nil
}

// SwapQuote requests a DEX swap quote.
func (c *Client) SwapQuote(walletID string, params map[string]string) (map[string]interface{}, error) {
	q := url.Values{}
	q.Set("wallet_id", walletID)
	q.Set("from_token", params["from_token"])
	q.Set("to_token", params["to_token"])
	q.Set("amount", params["amount"])
	if chain, ok := params["chain"]; ok {
		q.Set("chain", chain)
	} else {
		q.Set("chain", "base")
	}
	res, err := c.request("GET", "/api/swap/quote", nil, q)
	if err != nil {
		return nil, err
	}
	m, _ := res.(map[string]interface{})
	return m, nil
}

// AIGenerate sends a prompt to the AI Gateway.
func (c *Client) AIGenerate(walletID, model, prompt string) (map[string]interface{}, error) {
	res, err := c.request("POST", "/api/ai/generate", map[string]string{
		"wallet_id": walletID,
		"model":     model,
		"prompt":    prompt,
	}, nil)
	if err != nil {
		return nil, err
	}
	m, _ := res.(map[string]interface{})
	return m, nil
}
