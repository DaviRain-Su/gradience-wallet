package io.gradience.sdk;

import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import okhttp3.mockwebserver.RecordedRequest;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;

import java.util.List;
import java.util.Map;

import static org.junit.Assert.*;

public class GradienceClientTest {
    private MockWebServer server;
    private GradienceClient client;

    @Before
    public void setUp() throws Exception {
        server = new MockWebServer();
        server.start();
        client = new GradienceClient(server.url("/").toString(), "test-token");
    }

    @After
    public void tearDown() throws Exception {
        server.shutdown();
    }

    private void enqueueJson(int code, String body) {
        server.enqueue(new MockResponse()
                .setResponseCode(code)
                .setHeader("Content-Type", "application/json")
                .setBody(body));
    }

    @Test
    public void testCreateWallet() throws Exception {
        enqueueJson(201, "{\"id\":\"w1\",\"name\":\"demo\",\"status\":\"active\",\"created_at\":\"2026-01-01T00:00:00Z\"}");
        Wallet wallet = client.createWallet("demo");
        assertEquals("w1", wallet.id);
        assertEquals("demo", wallet.name);
    }

    @Test
    public void testListWallets() throws Exception {
        enqueueJson(200, "[{\"id\":\"w1\",\"name\":\"demo\",\"status\":\"active\",\"created_at\":\"2026-01-01T00:00:00Z\"}]");
        List<Wallet> wallets = client.listWallets();
        assertEquals(1, wallets.size());
        assertEquals("w1", wallets.get(0).id);
    }

    @Test
    public void testGetBalance() throws Exception {
        enqueueJson(200, "[{\"chain_id\":\"eip155:8453\",\"token_address\":\"0x0\",\"balance\":\"1000\",\"decimals\":6}]");
        List<Balance> balances = client.getBalance("w1");
        assertEquals("1000", balances.get(0).balance);
    }

    @Test
    public void testFundWallet() throws Exception {
        enqueueJson(200, "{\"txHash\":\"0xabc\"}");
        Map<String, Object> res = client.fundWallet("w1", "0xto", "1", "base");
        assertEquals("0xabc", res.get("txHash"));
    }

    @Test
    public void testSwapQuote() throws Exception {
        enqueueJson(200, "{\"from_token\":\"0xA\",\"to_token\":\"0xB\",\"from_amount\":\"1000\",\"to_amount\":\"2000\",\"chain\":\"base\"}");
        Map<String, Object> res = client.swapQuote("w1", "0xA", "0xB", "1000", "base");
        assertEquals("2000", res.get("to_amount"));
    }

    @Test
    public void testAIGenerate() throws Exception {
        enqueueJson(200, "{\"text\":\"hello\",\"cost\":\"0.001\"}");
        Map<String, Object> res = client.aiGenerate("w1", "claude", "hi");
        assertEquals("hello", res.get("text"));
    }

    @Test(expected = GradienceException.class)
    public void testErrorResponse() throws Exception {
        enqueueJson(400, "{\"error\":\"bad request\"}");
        client.createWallet("");
    }
}
