/**
 * Base x402 Node.js Bridge
 *
 * Wraps @x402/fetch + @x402/evm/exact/client to handle EVM x402 payments.
 * Input: JSON string via process.argv[2] with:
 *   { privateKey: "0x...", network: "eip155:84532", url: "...", method?: "GET", headers?: {}, body?: string }
 * Output: JSON with { status, headers, body }
 */
import { x402Client, wrapFetchWithPayment } from "@x402/fetch";
import { ExactEvmScheme } from "@x402/evm/exact/client";
import { privateKeyToAccount } from "viem/accounts";

const input = JSON.parse(process.argv[2] || "{}");
const { privateKey, network, url, method = "GET", headers = {}, body } = input;

if (!privateKey || !network || !url) {
  console.error(
    JSON.stringify({
      success: false,
      error: "Missing required fields: privateKey, network, url",
    })
  );
  process.exit(1);
}

const account = privateKeyToAccount(privateKey);
const scheme = new ExactEvmScheme(account);
const client = new x402Client().register(network, scheme);
const fetchWithPayment = wrapFetchWithPayment(fetch, client);

const init = { method };
if (Object.keys(headers).length > 0) {
  init.headers = headers;
}
if (body) {
  init.body = body;
}

try {
  const response = await fetchWithPayment(url, init);
  const responseHeaders = Object.fromEntries(response.headers.entries());
  const responseBody = await response.text();

  console.log(
    JSON.stringify({
      success: true,
      status: response.status,
      headers: responseHeaders,
      body: responseBody,
    })
  );
} catch (error) {
  console.error(
    JSON.stringify({
      success: false,
      error: error?.message || String(error),
    })
  );
  process.exit(1);
}
