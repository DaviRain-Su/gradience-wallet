import requests
from typing import Any, Dict, List, Optional


class GradienceError(Exception):
    def __init__(self, message: str, status_code: Optional[int] = None, body: Optional[Any] = None):
        super().__init__(message)
        self.status_code = status_code
        self.body = body


class GradienceClient:
    def __init__(self, base_url: str, api_token: Optional[str] = None):
        self.base_url = base_url.rstrip("/")
        self.api_token = api_token

    def _headers(self) -> Dict[str, str]:
        h: Dict[str, str] = {"Content-Type": "application/json"}
        if self.api_token:
            h["Authorization"] = f"Bearer {self.api_token}"
        return h

    def _request(self, method: str, path: str, **kwargs) -> Any:
        url = f"{self.base_url}{path}"
        try:
            resp = requests.request(method, url, headers=self._headers(), timeout=30, **kwargs)
        except requests.RequestException as e:
            raise GradienceError(str(e))
        try:
            data = resp.json()
        except Exception:
            data = resp.text
        if not resp.ok:
            raise GradienceError(
                data.get("error") if isinstance(data, dict) else str(data),
                status_code=resp.status_code,
                body=data,
            )
        return data

    def create_wallet(self, name: str) -> Dict[str, Any]:
        return self._request("POST", "/api/wallets", json={"name": name})

    def list_wallets(self) -> List[Dict[str, Any]]:
        return self._request("GET", "/api/wallets")

    def get_balance(self, wallet_id: str) -> List[Dict[str, Any]]:
        return self._request("GET", f"/api/wallets/{wallet_id}/balance")

    def fund_wallet(self, wallet_id: str, to: str, amount: str, chain: str = "base") -> Dict[str, Any]:
        return self._request(
            "POST",
            f"/api/wallets/{wallet_id}/fund",
            json={"to": to, "amount": amount, "chain": chain},
        )

    def sign_transaction(self, wallet_id: str, transaction: Dict[str, Any]) -> Dict[str, Any]:
        return self._request(
            "POST",
            f"/api/wallets/{wallet_id}/sign",
            json={"transaction": transaction},
        )

    def list_transactions(self, wallet_id: str) -> List[Dict[str, Any]]:
        return self._request("GET", f"/api/wallets/{wallet_id}/transactions")

    def swap_quote(self, wallet_id: str, params: Dict[str, Any]) -> Dict[str, Any]:
        query = {
            "wallet_id": wallet_id,
            "from_token": params.get("from_token"),
            "to_token": params.get("to_token"),
            "amount": params.get("amount"),
            "chain": params.get("chain", "base"),
        }
        return self._request("GET", "/api/swap/quote", params=query)

    def get_ai_balance(self, wallet_id: str) -> Dict[str, Any]:
        return self._request("GET", f"/api/ai/balance/{wallet_id}")

    def ai_generate(self, wallet_id: str, model: str, prompt: str) -> Dict[str, Any]:
        return self._request(
            "POST",
            "/api/ai/generate",
            json={"wallet_id": wallet_id, "model": model, "prompt": prompt},
        )

    def list_policies(self, wallet_id: str) -> List[Dict[str, Any]]:
        return self._request("GET", f"/api/wallets/{wallet_id}/policies")

    def create_policy(self, wallet_id: str, content: str) -> Dict[str, Any]:
        return self._request(
            "POST",
            f"/api/wallets/{wallet_id}/policies",
            json={"content": content},
        )

    def create_workspace_policy(self, workspace_id: str, content: str) -> Dict[str, Any]:
        return self._request(
            "POST",
            f"/api/workspaces/{workspace_id}/policies",
            json={"content": content},
        )

    def export_audit(self, wallet_id: str, fmt: str = "json") -> Any:
        return self._request(
            "GET",
            f"/api/wallets/{wallet_id}/audit/export",
            params={"format": fmt},
        )
