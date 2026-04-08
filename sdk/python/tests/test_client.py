import json
import unittest
from unittest.mock import patch, MagicMock
from gradience_sdk import GradienceClient, GradienceError


class FakeResponse:
    def __init__(self, status_code, json_data=None, text=None):
        self.status_code = status_code
        self._json = json_data
        self._text = text or json.dumps(json_data)
        self.ok = status_code < 400

    def json(self):
        return self._json

    def text(self):
        return self._text


class TestGradienceClient(unittest.TestCase):
    def setUp(self):
        self.client = GradienceClient("http://localhost:8080", api_token="token")

    @patch("gradience_sdk.client.requests.request")
    def test_create_wallet(self, mock_request):
        mock_request.return_value = FakeResponse(201, {"id": "w1", "name": "demo", "status": "active", "created_at": "2026-01-01T00:00:00Z"})
        wallet = self.client.create_wallet("demo")
        self.assertEqual(wallet["id"], "w1")
        mock_request.assert_called_once()
        args = mock_request.call_args
        self.assertEqual(args[0][0], "POST")
        self.assertIn("/api/wallets", args[0][1])

    @patch("gradience_sdk.client.requests.request")
    def test_list_wallets(self, mock_request):
        mock_request.return_value = FakeResponse(200, [{"id": "w1", "name": "demo", "status": "active", "created_at": "2026-01-01T00:00:00Z"}])
        wallets = self.client.list_wallets()
        self.assertEqual(len(wallets), 1)
        self.assertEqual(wallets[0]["id"], "w1")

    @patch("gradience_sdk.client.requests.request")
    def test_get_balance(self, mock_request):
        mock_request.return_value = FakeResponse(200, [{"chain_id": "eip155:8453", "token_address": "0x0", "balance": "1000", "decimals": 6}])
        balances = self.client.get_balance("w1")
        self.assertEqual(balances[0]["balance"], "1000")

    @patch("gradience_sdk.client.requests.request")
    def test_fund_wallet(self, mock_request):
        mock_request.return_value = FakeResponse(200, {"txHash": "0xabc"})
        res = self.client.fund_wallet("w1", "0xto", "1", "base")
        self.assertEqual(res["txHash"], "0xabc")

    @patch("gradience_sdk.client.requests.request")
    def test_swap_quote(self, mock_request):
        mock_request.return_value = FakeResponse(200, {"from_token": "0xA", "to_token": "0xB", "from_amount": "1000", "to_amount": "2000", "chain": "base"})
        quote = self.client.swap_quote("w1", {"from_token": "0xA", "to_token": "0xB", "amount": "1000", "chain": "base"})
        self.assertEqual(quote["to_amount"], "2000")

    @patch("gradience_sdk.client.requests.request")
    def test_ai_generate(self, mock_request):
        mock_request.return_value = FakeResponse(200, {"text": "hello", "cost": "0.001"})
        res = self.client.ai_generate("w1", "claude", "hi")
        self.assertEqual(res["text"], "hello")

    @patch("gradience_sdk.client.requests.request")
    def test_error_raises_gradience_error(self, mock_request):
        mock_request.return_value = FakeResponse(400, {"error": "bad request"})
        with self.assertRaises(GradienceError) as ctx:
            self.client.create_wallet("")
        self.assertEqual(ctx.exception.status_code, 400)
        self.assertEqual(str(ctx.exception), "bad request")

    @patch("gradience_sdk.client.requests.request")
    def test_create_policy(self, mock_request):
        mock_request.return_value = FakeResponse(200, {"policy_id": "p1"})
        res = self.client.create_policy("w1", "{}")
        self.assertEqual(res["policy_id"], "p1")

    @patch("gradience_sdk.client.requests.request")
    def test_export_audit(self, mock_request):
        mock_request.return_value = FakeResponse(200, [{"id": "a1", "action": "sign"}])
        logs = self.client.export_audit("w1", "json")
        self.assertIsInstance(logs, list)


if __name__ == "__main__":
    unittest.main()
