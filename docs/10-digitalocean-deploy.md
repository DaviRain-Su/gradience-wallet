# Deploy Gradience API to DigitalOcean Droplet

This guide shows how to deploy the Rust backend API to a DigitalOcean Droplet (or any Ubuntu VPS) using Docker Compose.

> **Important for Passkeys**: WebAuthn / Passkey requires a **registrable domain** as `RP_ID`. IP addresses will not work for Passkey registration on most browsers. You need a custom domain pointed to your Droplet.

---

## Option A — Quick Start (No Domain, HTTP Only)

Use this if you just want to test the API quickly. **Passkey registration will fail** without a domain.

### 1. SSH into your Droplet

```bash
ssh root@your-droplet-ip
```

### 2. Install Docker & Docker Compose

```bash
apt-get update && apt-get install -y docker.io docker-compose
```

### 3. Clone the repo

```bash
git clone https://github.com/DaviRain-Su/gradience-wallet.git
cd gradience-wallet
```

### 4. Start the API

```bash
ORIGIN=http://your-droplet-ip:8080 \
RP_ID=your-droplet-ip \
docker-compose up -d --build
```

The API will be available at `http://your-droplet-ip:8080`.

---

## Option B — Production with Caddy + HTTPS (Recommended)

### Prerequisites

- A domain (or subdomain) pointing to your Droplet IP, e.g. `api.gradience.example.com`
- Open ports 80 and 443 on your Droplet firewall

### 1. SSH into your Droplet

```bash
ssh root@your-droplet-ip
```

### 2. Install Docker & Docker Compose

```bash
apt-get update && apt-get install -y docker.io docker-compose
```

### 3. Clone the repo

```bash
git clone https://github.com/DaviRain-Su/gradience-wallet.git
cd gradience-wallet
```

### 4. Set up Caddyfile

Edit `Caddyfile` to use your domain:

```
api.yourdomain.com {
    reverse_proxy api:8080
}
```

> Replace `api.yourdomain.com` with your actual domain.

### 5. Start with Caddy

```bash
ORIGIN=https://api.yourdomain.com \
RP_ID=api.yourdomain.com \
docker-compose -f docker-compose.caddy.yml up -d --build
```

Caddy will automatically obtain and renew Let's Encrypt certificates.

---

## Connect Vercel Frontend

1. Go to your Vercel project → **Settings** → **Environment Variables**
2. Add:
   ```
   NEXT_PUBLIC_API_URL=https://api.yourdomain.com
   ```
   (or `http://your-droplet-ip:8080` if using Option A)
3. Redeploy the frontend.

Also update the API environment and restart:

```bash
cd gradience-wallet
# Edit .env or pass inline
ORIGIN=https://your-frontend-url.vercel.app \
RP_ID=your-frontend-url.vercel.app \
docker-compose up -d
```

> `ORIGIN` must be the full frontend URL with `https://`.  
> `RP_ID` must be just the hostname (no `https://`, no trailing slash).

---

## Useful Commands

| Action | Command |
|--------|---------|
| View logs | `docker-compose logs -f api` |
| Restart API | `docker-compose restart api` |
| Update after git pull | `docker-compose up -d --build` |
| Backup SQLite | `cp data/gradience.db data/gradience.db.backup` |

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| "Passkey registration failed" | Ensure `RP_ID` matches your frontend hostname exactly |
| CORS errors from frontend | Ensure `ORIGIN` matches your Vercel frontend URL |
| Database resets on restart | Ensure `./data` directory is mounted as a volume |
