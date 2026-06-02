# Masday Remote Mode — Complete Guide

Masday supports two modes:
- **Local mode**: Everything runs on your machine (requires Rust toolchain)
- **Remote mode**: Server runs on a VPS, client connects remotely (no Rust needed on client)

This guide covers setting up a remote Masday server on a VPS and connecting clients.

---

## Architecture

```
┌─────────────────┐         ┌──────────────────────────────────────┐
│   Your Laptop    │         │          VPS (Remote Server)          │
│                  │  HTTPS  │                                      │
│  Claude Code ────┼────────►│  masday-api (port 30101)             │
│  masday CLI      │         │    └── REST API + WebSocket          │
│                  │         │                                      │
│  masday-mcp ─────┼────────►│  PostgreSQL 16 (port 54341)          │
│  (MCP protocol)  │  TCP    │    └── pgvector, 16 tables           │
│                  │         │                                      │
│                  │         │  Redis 7 (port 63791)                 │
│                  │         │    └── Caching, sessions              │
└─────────────────┘         └──────────────────────────────────────┘
```

The MCP server (`masday-mcp`) runs locally on the client machine and connects to the remote PostgreSQL database. The API server (`masday-api`) runs on the VPS for dashboard access.

---

## Part 1: VPS Server Setup

### Prerequisites

- Ubuntu 22.04+ or similar Linux
- 2GB+ RAM, 2+ vCPU
- Docker and Docker Compose installed

### Step 1: Clone and Build

```bash
# SSH into your VPS
ssh user@your-vps-ip

# Clone the repository
git clone https://github.com/dayartcrew-web/masday-workflow-rust.git
cd masday-workflow-rust

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build release binaries
cargo build --release -p masday-api
cargo build --release -p masday-mcp
```

### Step 2: Start Infrastructure

```bash
# Start PostgreSQL + Redis
docker compose up -d

# Verify PostgreSQL is running
docker compose ps
```

### Step 3: Configure Environment

```bash
# Create .env file
cat > .env << 'EOF'
# Database
DATABASE_URL=postgresql://trader:traderpass@localhost:54341/masday_workflow

# API Server
API_PORT=30101
API_HOST=0.0.0.0

# Embedding (local fastembed — no external service)
EMBEDDING_PROVIDER=local
EMBEDDING_MODEL=all-MiniLM-L6-v2
EMBEDDING_DIMENSIONS=384

# Logging
RUST_LOG=info
EOF
```

### Step 4: Run Migrations

```bash
# Tables auto-create on first API startup
# Or run manually:
source .env && cargo run -p masday-api
# Wait for "Server listening on 0.0.0.0:30101" then Ctrl+C
```

### Step 5: Start API Server

```bash
# Option A: Direct (for testing)
source .env && ./target/release/masday-api

# Option B: Systemd service (recommended for production)
sudo tee /etc/systemd/system/masday-api.service << EOF
[Unit]
Description=Masday API Server
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=$USER
WorkingDirectory=$(pwd)
EnvironmentFile=$(pwd)/.env
ExecStart=$(pwd)/target/release/masday-api
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable masday-api
sudo systemctl start masday-api
sudo systemctl status masday-api
```

### Step 6: Configure Firewall

```bash
# Allow API port
sudo ufw allow 30101/tcp

# IMPORTANT: Do NOT expose PostgreSQL (54341) or Redis (63791) to the internet!
# They should only accept connections from localhost.
# If you need remote MCP access, use SSH tunnel (see below).
```

### Step 7: Set Up HTTPS (Recommended)

```bash
# Install nginx as reverse proxy
sudo apt install nginx certbot python3-certbot-nginx -y

# Configure nginx
sudo tee /etc/nginx/sites-available/masday << EOF
server {
    listen 80;
    server_name masday.yourdomain.com;

    location / {
        proxy_pass http://127.0.0.1:30101;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}
EOF

sudo ln -s /etc/nginx/sites-available/masday /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx

# Get SSL certificate
sudo certbot --nginx -d masday.yourdomain.com
```

---

## Part 2: Remote Database Access

The MCP server needs direct PostgreSQL access. Two options:

### Option A: SSH Tunnel (Recommended — Secure)

No firewall changes needed. The MCP server on your laptop connects through SSH.

```bash
# On your laptop, create SSH tunnel
ssh -L 54341:localhost:54341 -N user@your-vps-ip &

# Verify tunnel works
psql postgresql://trader:traderpass@localhost:54341/masday_workflow -c "SELECT 1"
```

### Option B: Direct PostgreSQL Access (Less Secure)

```bash
# On VPS: Edit postgres config to allow remote connections
# In docker-compose.yml, add port mapping:
#   ports:
#     - "54341:5432"

# Configure pg_hba.conf for password auth
# Restrict to specific IP if possible
sudo ufw allow from YOUR_LAPTOP_IP to any port 54341
```

---

## Part 3: Client Setup (Your Laptop)

### Step 1: Download Binary

```bash
# Linux
curl -fsSL -o masday https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-linux-x86_64
chmod +x masday

# macOS (when available)
# curl -fsSL -o masday https://github.com/dayartcrew-web/masday-workflow-rust/releases/latest/download/masday-macos-aarch64
```

### Step 2: Install Remote Mode

```bash
cd /path/to/your/project

# Remote install — no Rust needed
./masday install --remote https://masday.yourdomain.com --api-key YOUR_API_KEY
```

This configures:
- MCP server to connect to remote database
- API health checks to remote server
- All hooks and agents locally

### Step 3: Configure MCP Connection

Edit `.mcp.json` in your project root:

```json
{
  "mcpServers": {
    "masday": {
      "command": "masday-mcp",
      "args": [],
      "cwd": "/path/to/your/project",
      "env": {
        "DATABASE_URL": "postgresql://trader:traderpass@localhost:54341/masday_workflow",
        "EMBEDDING_PROVIDER": "local",
        "EMBEDDING_MODEL": "all-MiniLM-L6-v2",
        "EMBEDDING_DIMENSIONS": "384"
      }
    }
  }
}
```

> **Note:** `DATABASE_URL` uses `localhost:54341` because of the SSH tunnel.

### Step 4: Start SSH Tunnel (if using tunnel)

```bash
# Add to ~/.bashrc or run before starting Claude Code
ssh-tunnel-masday() {
  ssh -L 54341:localhost:54341 -N user@your-vps-ip -o ServerAliveInterval=60 &
  echo "SSH tunnel active (PID: $!)"
  echo "Run 'kill $!' to stop"
}

# Auto-start on shell login (optional)
echo 'ssh -L 54341:localhost:54341 -N -f user@your-vps-ip' >> ~/.bashrc
```

### Step 5: Verify Connection

```bash
# Check API is reachable
curl https://masday.yourdomain.com/api/health

# Check database via tunnel
psql postgresql://trader:traderpass@localhost:54341/masday_workflow -c "SELECT count(*) FROM \"Workflow\""
```

---

## Part 4: Running MCP Server Locally (Client)

The MCP server runs on your laptop and connects to the remote database:

```bash
# Option A: Claude Code starts it automatically via .mcp.json
# Just start Claude Code — it reads .mcp.json and launches masday-mcp

# Option B: Manual start for testing
DATABASE_URL=postgresql://trader:traderpass@localhost:54341/masday_workflow \
  EMBEDDING_PROVIDER=local \
  masday-mcp
```

---

## Part 5: Security Checklist

| Item | Status |
|------|--------|
| PostgreSQL not exposed to internet | ✅ SSH tunnel or localhost only |
| Redis not exposed to internet | ✅ localhost only |
| API behind HTTPS (nginx + certbot) | ✅ |
| API key authentication | 🔜 Add middleware |
| Firewall blocks all except 80/443/22 | ✅ `ufw enable` |
| SSH key-based auth (no passwords) | ✅ `PasswordAuthentication no` |
| Docker containers not privileged | ✅ Default |
| Regular PostgreSQL backups | 🔜 Set up pg_dump cron |

### Set Up Backups

```bash
# Add daily backup cron
crontab -e
# Add:
# 0 3 * * * docker exec masday-postgres pg_dump -U trader masday_workflow | gzip > /backup/masday_$(date +\%Y\%m\%d).sql.gz
# 0 4 * * * find /backup -name "masday_*.sql.gz" -mtime +30 -delete
```

---

## Part 6: Monitoring

```bash
# Check API health
curl -s https://masday.yourdomain.com/api/health | jq

# Check PostgreSQL
docker exec -it masday-postgres psql -U trader -d masday_workflow -c "SELECT count(*) FROM \"Workflow\""

# Check logs
sudo journalctl -u masday-api -f

# Check resources
docker stats --no-stream
```

---

## Quick Reference: Remote Mode Commands

```bash
# ── VPS (Server) ───────────────────────────────────────
# Start everything
docker compose up -d
sudo systemctl start masday-api

# Check status
sudo systemctl status masday-api
curl http://localhost:30101/api/health

# ── Laptop (Client) ────────────────────────────────────
# SSH tunnel
ssh -L 54341:localhost:54341 -N -f user@your-vps-ip

# Install masday into project
masday install --remote https://masday.yourdomain.com

# Verify
curl https://masday.yourdomain.com/api/health
psql $DATABASE_URL -c "SELECT 1"
```

---

## Troubleshooting

### "Connection refused" on port 54341

SSH tunnel not running:
```bash
# Check if tunnel is active
ss -tlnp | grep 54341

# Restart tunnel
pkill -f "ssh -L 54341"
ssh -L 54341:localhost:54341 -N -f user@your-vps-ip
```

### MCP server not starting

Check `.mcp.json` DATABASE_URL matches your tunnel:
```bash
cat .mcp.json | grep DATABASE_URL
```

### Embedding model download fails

First-time download needs internet:
```bash
# Pre-download model on VPS
EMBEDDING_PROVIDER=local cargo run -p masday-mcp
# Wait for "Local embedding model loaded successfully" then Ctrl+C
```

### API returns 502

nginx can't reach the API:
```bash
sudo systemctl status masday-api
curl http://localhost:30101/api/health
sudo nginx -t
```
