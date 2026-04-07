# Gradience Wallet — Deployment Guide

This guide walks you through deploying Gradience Wallet so anyone can use it from anywhere.

---

## Architecture

| Component | Technology | Deployment Target |
|-----------|------------|-------------------|
| Web UI    | Next.js 14 | Vercel            |
| API       | Rust (Axum)| Railway / Fly.io  |
| Database  | SQLite     | Persistent volume |

---

## 1. Deploy the API (Backend)

### Option A — Railway (Recommended for beginners)

1. Push this repo to GitHub.
2. Go to [railway.app](https://railway.app), click **New Project** → **Deploy from GitHub repo**.
3. Add a **New Service** → **Dockerfile**.
4. Set the service **Root Directory** to `/` (repo root).
5. In **Variables**, add:
   ```
   DATABASE_URL=sqlite:/app/data/gradience.db?mode=rwc
   GRADIENCE_DATA_DIR=/app/data
   ORIGIN=https://your-frontend-url.vercel.app
   RP_ID=your-frontend-url.vercel.app
   ANCHOR_INTERVAL_SEC=300
   ```
   > **Note:** `RP_ID` must be a registrable domain. Vercel preview URLs (`*.vercel.app`) work, but custom domains are preferred for Passkey stability.
6. Add a **Volume** mounted at `/app/data` so SQLite persists across deploys.
7. Generate a domain in Railway → copy the URL for step 2.

### Option B — Fly.io

```bash
# Install flyctl first: https://fly.io/docs/hands-on/install-flyctl/
fly launch --dockerfile Dockerfile --name gradience-api
fly volume create gradience_data --size 1
fly secrets set DATABASE_URL="sqlite:/app/data/gradience.db?mode=rwc"
fly secrets set GRADIENCE_DATA_DIR="/app/data"
fly secrets set ORIGIN="https://wallet.example.com"
fly secrets set RP_ID="wallet.example.com"
```

---

## 2. Deploy the Web UI (Frontend)

### Vercel

1. Go to [vercel.com](https://vercel.com), import this GitHub repo.
2. Set **Root Directory** to `web`.
3. In **Environment Variables**, add:
   ```
   NEXT_PUBLIC_API_URL=https://your-railway-or-fly-url.up.railway.app
   ```
4. Deploy.
5. Copy the Vercel production URL.

---

## 3. Update API Environment Variables

After you get the Vercel frontend URL, go back to your API deployment and update:

```
ORIGIN=https://your-vercel-production-url.vercel.app
RP_ID=your-vercel-production-url.vercel.app
```

Then **redeploy** the API so Passkey challenges are bound to the correct domain.

---

## 4. WebAuthn / Passkey Checklist

Passkeys are strict about domains. Make sure:

- `RP_ID` matches the **hostname** of your frontend (no `https://` prefix, no trailing slash).
- `ORIGIN` includes the **full URL** with `https://`.
- If you use a custom domain (e.g. `wallet.gradience.xyz`), set both `ORIGIN` and `RP_ID` to that domain.
- `localhost` only works on `http://localhost` / `http://127.0.0.1`. Once deployed, you must use HTTPS.

---

## 5. Smoke Test

1. Open your Vercel URL.
2. Register a new account with Passkey.
3. Log out, then log back in.
4. Create a wallet, check balance, try a swap or fund.
5. Run the MCP client or embedded-wallet example against the live API.

---

## 6. (Optional) Custom Domain

For a polished hackathon demo, buy a cheap domain and point both Vercel and your API host to it:

- **Frontend**: `wallet.yourdomain.com` → CNAME to `cname.vercel-dns.com`
- **API**: `api.yourdomain.com` → CNAME to your Railway/Fly domain

Update `NEXT_PUBLIC_API_URL`, `ORIGIN`, and `RP_ID` accordingly.

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| "Passkey registration failed" | `RP_ID` / `ORIGIN` mismatch | Check env vars and redeploy API |
| "Network error" from frontend | API URL wrong or CORS issue | Verify `NEXT_PUBLIC_API_URL` and that API is running |
| Database resets on redeploy | Volume not mounted | Ensure `/app/data` is a persistent volume |
| OAuth callbacks fail | Hardcoded `localhost:3000` | Update `oauth_start` redirect URIs in `crates/gradience-api/src/main.rs` |
