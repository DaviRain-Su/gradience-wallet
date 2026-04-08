import { NeynarAPIClient } from "@neynar/nodejs-sdk";
import { config } from "dotenv";
import { spawn } from "child_process";
import * as path from "path";

config();

const NEYNAR_API_KEY = process.env.NEYNAR_API_KEY!;
const BOT_FID = parseInt(process.env.BOT_FID || "0", 10);
const BOT_USERNAME = process.env.BOT_USERNAME || "gradience_bot";
const WALLET_ID = process.env.WALLET_ID || "367147d7-0215-4b68-b694-38ff9c60a3da";
const CHAIN = process.env.CHAIN || "eip155:84532";
const PAY_TOKEN = process.env.PAY_TOKEN || "0x036CbD53842c5426634e7929541eC2318f3dCF7e";
const BRIDGE_DIR = process.env.BASE_X402_BRIDGE_DIR || "../../bridge/base-x402";
const POLL_INTERVAL = parseInt(process.env.POLL_INTERVAL_SECONDS || "30", 10) * 1000;

if (!NEYNAR_API_KEY || !BOT_FID) {
  console.error("Missing NEYNAR_API_KEY or BOT_FID");
  process.exit(1);
}

const neynar = new NeynarAPIClient({ apiKey: NEYNAR_API_KEY });
let lastCheckedTimestamp = Math.floor(Date.now() / 1000);

function deriveDemoSeed(walletId: string, chain: string, derivationPath: string): Buffer {
  const { createHash } = require("crypto");
  return createHash("sha3-256")
    .update(walletId)
    .update(chain)
    .update(derivationPath)
    .digest();
}

function runBaseBridge(input: object): Promise<{ success: boolean; status?: number; headers?: Record<string, string>; body?: string; error?: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn("node", ["index.mjs", JSON.stringify(input)], {
      cwd: path.resolve(BRIDGE_DIR),
      env: { ...process.env, NODE_NO_WARNINGS: "1" },
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (d) => { stdout += d; });
    child.stderr.on("data", (d) => { stderr += d; });
    child.on("close", (code) => {
      if (code !== 0) {
        return resolve({ success: false, error: stderr || "bridge exited with code " + code });
      }
      try {
        resolve(JSON.parse(stdout));
      } catch {
        resolve({ success: false, error: "invalid json: " + stdout });
      }
    });
    child.on("error", (err) => reject(err));
  });
}

async function handleMention(cast: any) {
  const text: string = cast.text || "";
  const authorFid = cast.author.fid;
  const castHash = cast.hash;

  console.log(`[${new Date().toISOString()}] Mention from @${cast.author.username}: ${text}`);

  // Match patterns like:
  // @gradience_bot pay https://example.com/weather
  // @gradience_bot pay http://localhost:4021/weather
  const payRegex = new RegExp(`@${BOT_USERNAME}\\s+pay\\s+(https?://\\S+)`, "i");
  const match = text.match(payRegex);

  if (!match) {
    console.log("No pay command detected.");
    return;
  }

  const url = match[1];
  console.log(`Initiating x402 payment for URL: ${url}`);

  const derivationPath = "m/44'/60'/0'/0/0";
  const seed = deriveDemoSeed(WALLET_ID, CHAIN, derivationPath);
  const privateKey = "0x" + seed.toString("hex");

  const bridgeResult = await runBaseBridge({
    privateKey,
    network: CHAIN,
    url,
    method: "GET",
    headers: {},
    body: null,
  });

  let replyText: string;
  if (bridgeResult.success && bridgeResult.status === 200) {
    const paymentResponseHeader = bridgeResult.headers?.["payment-response"] || "";
    let txHash = "";
    if (paymentResponseHeader) {
      try {
        const decoded = Buffer.from(paymentResponseHeader, "base64").toString("utf8");
        const parsed = JSON.parse(decoded);
        txHash = parsed.transaction || "";
      } catch {
        // ignore
      }
    }
    replyText = `✅ Paid via Gradience!\n` +
      `tx: ${txHash ? `https://sepolia.basescan.org/tx/${txHash}` : "N/A"}\n` +
      `body: ${(bridgeResult.body || "").slice(0, 200)}`;
  } else if (bridgeResult.status === 402) {
    const paymentRequiredHeader = bridgeResult.headers?.["payment-required"] || "";
    let errorMsg = "insufficient balance or missing USDC";
    if (paymentRequiredHeader) {
      try {
        const decoded = Buffer.from(paymentRequiredHeader, "base64").toString("utf8");
        const parsed = JSON.parse(decoded);
        if (parsed.error) errorMsg = parsed.error;
      } catch {
        // ignore
      }
    }
    replyText = `⚠️ Payment failed: ${errorMsg}\n` +
      `Make sure the payer wallet has Base Sepolia USDC.\n` +
      `Faucet: https://portal.cdp.coinbase.com/products/faucet`;
  } else {
    replyText = `❌ Payment failed: ${bridgeResult.error || "unknown error"}`;
  }

  try {
    await neynar.publishCast({
      signerUuid: process.env.NEYNAR_SIGNER_UUID || "",
      text: replyText,
      parent: castHash,
    });
    console.log("Reply sent successfully.");
  } catch (err: any) {
    console.error("Failed to send reply:", err?.response?.data || err.message);
  }
}

async function pollMentions() {
  try {
    // Fetch notifications (mentions + replies) for the bot
    const response: any = await neynar.fetchAllNotifications({
      fid: BOT_FID,
      type: ["mentions" as any],
    });

    const notifications = response.notifications || [];

    for (const notification of notifications) {
      const cast = notification.cast;
      if (!cast) continue;
      const timestamp = new Date(cast.timestamp).getTime() / 1000;
      if (timestamp <= lastCheckedTimestamp) continue;

      // Only handle direct mentions (not replies to our own casts unless they mention us)
      if (cast.mentionedProfiles?.some((p: any) => p.fid === BOT_FID)) {
        await handleMention(cast);
      }
    }

    if (notifications.length > 0) {
      const newest = notifications[0].cast;
      if (newest) {
        lastCheckedTimestamp = Math.max(
          lastCheckedTimestamp,
          new Date(newest.timestamp).getTime() / 1000
        );
      }
    }
  } catch (err: any) {
    console.error("Polling error:", err?.response?.data || err.message);
  }

  setTimeout(pollMentions, POLL_INTERVAL);
}

console.log(`Gradience Farcaster Bot starting...`);
console.log(`Bot FID: ${BOT_FID}, Username: ${BOT_USERNAME}`);
console.log(`Polling mentions every ${POLL_INTERVAL / 1000}s`);

pollMentions();
