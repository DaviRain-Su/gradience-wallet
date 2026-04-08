/**
 * Gradience Wallet — Stellar x402 Bridge
 *
 * This Node.js bridge uses the official @x402/stellar client library to handle
 * Soroban auth-entry signing for x402 payments. Gradience API (Rust) invokes
 * this bridge as a subprocess, passing the Stellar secret key and the 402
 * response; the bridge returns the payment signature headers.
 */

import { createEd25519Signer } from "@x402/stellar";
import { ExactStellarScheme } from "@x402/stellar/exact/client";
import { x402Client, x402HTTPClient } from "@x402/core/client";

async function run() {
  const raw = process.argv[2];
  if (!raw) {
    console.error("Usage: node index.mjs '<json_input>'");
    process.exit(1);
  }

  let input;
  try {
    input = JSON.parse(raw);
  } catch (e) {
    console.error("Invalid JSON input:", e.message);
    process.exit(1);
  }

  const { privateKey, network, response } = input;
  if (!privateKey || !network || !response) {
    console.error("Missing required fields: privateKey, network, response");
    process.exit(1);
  }

  try {
    const signer = createEd25519Signer(privateKey, network);
    const coreClient = new x402Client().register("stellar:*", new ExactStellarScheme(signer));
    const client = new x402HTTPClient(coreClient);

    const getHeader = (name) => {
      const key = name.toLowerCase();
      for (const [h, v] of Object.entries(response.headers ?? {})) {
        if (h.toLowerCase() === key) {
          return String(v);
        }
      }
      return null;
    };

    const paymentRequired = client.getPaymentRequiredResponse(
      getHeader,
      response.body ?? null,
    );

    const payload = await client.createPaymentPayload(paymentRequired);
    const headers = client.encodePaymentSignatureHeader(payload);

    console.log(JSON.stringify({ success: true, headers }));
  } catch (err) {
    console.error(
      JSON.stringify({
        success: false,
        error: err?.message ?? String(err),
      }),
    );
    process.exit(1);
  }
}

run();
