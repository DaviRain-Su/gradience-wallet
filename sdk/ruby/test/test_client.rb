require "minitest/autorun"
require "json"
require "webrick"
require_relative "../lib/gradience"

class TestClient < Minitest::Test
  def setup
    @server = WEBrick::HTTPServer.new(Port: 0, Logger: WEBrick::Log.new("/dev/null"), AccessLog: [])
    @server.mount_proc "/api/wallets" do |req, res|
      res.content_type = "application/json"
      if req.request_method == "POST"
        res.body = { id: "w1", name: "demo", status: "active", created_at: "2026-01-01T00:00:00Z" }.to_json
      else
        res.body = [{ id: "w1", name: "demo", status: "active", created_at: "2026-01-01T00:00:00Z" }].to_json
      end
    end
    @server.mount_proc "/api/wallets/w1/balance" do |req, res|
      res.content_type = "application/json"
      res.body = [{ chain_id: "eip155:8453", token_address: "0x0", balance: "1000", decimals: 6 }].to_json
    end
    @server.mount_proc "/api/wallets/w1/fund" do |req, res|
      res.content_type = "application/json"
      res.body = { txHash: "0xabc" }.to_json
    end
    @server.mount_proc "/api/swap/quote" do |req, res|
      res.content_type = "application/json"
      res.body = { from_token: "0xA", to_token: "0xB", from_amount: "1000", to_amount: "2000", chain: "base" }.to_json
    end
    @server.mount_proc "/api/ai/generate" do |req, res|
      res.content_type = "application/json"
      res.body = { text: "hello", cost: "0.001" }.to_json
    end
    @server.mount_proc "/api/wallets" do |req, res|
      res.content_type = "application/json"
      if req.request_method == "POST"
        body = JSON.parse(req.body)
        if body["name"].nil? || body["name"].empty?
          res.status = 400
          res.body = { error: "bad request" }.to_json
        else
          res.body = { id: "w1", name: "demo", status: "active", created_at: "2026-01-01T00:00:00Z" }.to_json
        end
      else
        res.body = [{ id: "w1", name: "demo", status: "active", created_at: "2026-01-01T00:00:00Z" }].to_json
      end
    end
    Thread.new { @server.start }
    @port = @server.config[:Port]
    @client = Gradience::Client.new("http://127.0.0.1:#{@port}", api_token: "token")
  end

  def teardown
    @server.shutdown
  end

  def test_create_wallet
    wallet = @client.create_wallet("demo")
    assert_equal "w1", wallet["id"]
    assert_equal "demo", wallet["name"]
  end

  def test_list_wallets
    wallets = @client.list_wallets
    assert_equal 1, wallets.length
    assert_equal "w1", wallets[0]["id"]
  end

  def test_get_balance
    balances = @client.get_balance("w1")
    assert_equal "1000", balances[0]["balance"]
  end

  def test_fund_wallet
    res = @client.fund_wallet("w1", to: "0xto", amount: "1", chain: "base")
    assert_equal "0xabc", res["txHash"]
  end

  def test_swap_quote
    res = @client.swap_quote("w1", from_token: "0xA", to_token: "0xB", amount: "1000", chain: "base")
    assert_equal "2000", res["to_amount"]
  end

  def test_ai_generate
    res = @client.ai_generate("w1", model: "claude", prompt: "hi")
    assert_equal "hello", res["text"]
  end

  def test_error_response
    assert_raises(Gradience::Error) do
      @client.create_wallet("")
    end
  end
end
