module Gradience
  class Client
    def initialize(base_url, api_token: nil)
      @base_url = base_url.chomp("/")
      @api_token = api_token
    end

    def create_wallet(name)
      request(:post, "/api/wallets", body: { name: name })
    end

    def list_wallets
      request(:get, "/api/wallets")
    end

    def get_balance(wallet_id)
      request(:get, "/api/wallets/#{wallet_id}/balance")
    end

    def fund_wallet(wallet_id, to:, amount:, chain: "base")
      request(:post, "/api/wallets/#{wallet_id}/fund", body: { to: to, amount: amount, chain: chain })
    end

    def swap_quote(wallet_id, from_token:, to_token:, amount:, chain: "base")
      query = {
        wallet_id: wallet_id,
        from_token: from_token,
        to_token: to_token,
        amount: amount,
        chain: chain
      }
      request(:get, "/api/swap/quote", query: query)
    end

    def ai_generate(wallet_id, model:, prompt:)
      request(:post, "/api/ai/generate", body: { wallet_id: wallet_id, model: model, prompt: prompt })
    end

    private

    def request(method, path, body: nil, query: nil)
      uri = URI.parse("#{@base_url}#{path}")
      if query
        uri.query = URI.encode_www_form(query)
      end

      req = Net::HTTP.const_get(method.to_s.capitalize).new(uri)
      req["Content-Type"] = "application/json"
      req["Authorization"] = "Bearer #{@api_token}" if @api_token
      req.body = body.to_json if body

      response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: uri.scheme == "https") do |http|
        http.request(req)
      end

      data = parse_body(response)

      unless response.is_a?(Net::HTTPSuccess)
        msg = data.is_a?(Hash) && data["error"] ? data["error"] : data.to_s
        raise Error.new(msg, response.code.to_i, data)
      end

      data
    end

    def parse_body(response)
      body = response.body.to_s
      JSON.parse(body)
    rescue JSON::ParserError
      body
    end
  end
end
