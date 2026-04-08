package io.gradience.sdk;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;
import okhttp3.*;

import java.io.IOException;
import java.util.List;
import java.util.Map;

public class GradienceClient {
    private final String baseUrl;
    private final String apiToken;
    private final OkHttpClient httpClient;
    private final Gson gson = new Gson();

    public GradienceClient(String baseUrl, String apiToken) {
        this.baseUrl = baseUrl.replaceAll("/$", "");
        this.apiToken = apiToken;
        this.httpClient = new OkHttpClient();
    }

    private Headers headers() {
        Headers.Builder h = new Headers.Builder();
        h.add("Content-Type", "application/json");
        if (apiToken != null && !apiToken.isEmpty()) {
            h.add("Authorization", "Bearer " + apiToken);
        }
        return h.build();
    }

    private String request(String method, String path, Object body, HttpUrl.Builder urlBuilder) throws GradienceException {
        if (urlBuilder == null) {
            urlBuilder = HttpUrl.parse(baseUrl + path).newBuilder();
        }
        HttpUrl url = urlBuilder.build();

        Request.Builder req = new Request.Builder().url(url).headers(headers());
        if (body != null) {
            req.method(method, RequestBody.create(gson.toJson(body), MediaType.parse("application/json")));
        } else {
            req.method(method, null);
        }

        try (Response resp = httpClient.newCall(req.build()).execute()) {
            String respBody = resp.body() != null ? resp.body().string() : "";
            if (!resp.isSuccessful()) {
                throw new GradienceException(parseError(respBody), resp.code(), respBody);
            }
            return respBody;
        } catch (IOException e) {
            throw new GradienceException(e.getMessage(), null, null);
        }
    }

    private String parseError(String body) {
        try {
            Map<String, Object> m = gson.fromJson(body, new TypeToken<Map<String, Object>>() {}.getType());
            if (m != null && m.get("error") instanceof String) {
                return (String) m.get("error");
            }
        } catch (Exception ignored) {}
        return body;
    }

    public Wallet createWallet(String name) throws GradienceException {
        String json = request("POST", "/api/wallets", Map.of("name", name), null);
        return gson.fromJson(json, Wallet.class);
    }

    public List<Wallet> listWallets() throws GradienceException {
        String json = request("GET", "/api/wallets", null, null);
        return gson.fromJson(json, new TypeToken<List<Wallet>>() {}.getType());
    }

    public List<Balance> getBalance(String walletId) throws GradienceException {
        String json = request("GET", "/api/wallets/" + walletId + "/balance", null, null);
        return gson.fromJson(json, new TypeToken<List<Balance>>() {}.getType());
    }

    public Map<String, Object> fundWallet(String walletId, String to, String amount, String chain) throws GradienceException {
        String json = request("POST", "/api/wallets/" + walletId + "/fund", Map.of("to", to, "amount", amount, "chain", chain), null);
        return gson.fromJson(json, new TypeToken<Map<String, Object>>() {}.getType());
    }

    public Map<String, Object> swapQuote(String walletId, String fromToken, String toToken, String amount, String chain) throws GradienceException {
        HttpUrl.Builder ub = HttpUrl.parse(baseUrl + "/api/swap/quote").newBuilder();
        ub.addQueryParameter("wallet_id", walletId);
        ub.addQueryParameter("from_token", fromToken);
        ub.addQueryParameter("to_token", toToken);
        ub.addQueryParameter("amount", amount);
        ub.addQueryParameter("chain", chain != null ? chain : "base");
        String json = request("GET", "/api/swap/quote", null, ub);
        return gson.fromJson(json, new TypeToken<Map<String, Object>>() {}.getType());
    }

    public Map<String, Object> aiGenerate(String walletId, String model, String prompt) throws GradienceException {
        String json = request("POST", "/api/ai/generate", Map.of("wallet_id", walletId, "model", model, "prompt", prompt), null);
        return gson.fromJson(json, new TypeToken<Map<String, Object>>() {}.getType());
    }
}
