# Running a Node

Complete guide to operating a StreamSync node.

---

## Prerequisites

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| **CPU** | 8 cores | 32 cores |
| **RAM** | 32 GB | 128 GB |
| **Storage** | 500 GB SSD | 4 TB NVMe |
| **Network** | 100 Mbps | 10 Gbps |
| **Stake** | 10,000 STRM | 50,000+ STRM |

---

## Quick Start

```bash
# 1. Build
git clone https://github.com/your-org/streamsync.git
cd streamsync
cargo build --release

# 2. Initialize
./target/release/streamsync init --config node.toml

# 3. Configure (edit node.toml)
vim node.toml

# 4. Stake tokens
streamsync stake 10000

# 5. Run
./target/release/streamsync run --config node.toml
```

---

## Configuration

### Essential Settings

```toml
[node]
id = "my-node-001"
type = "speed-runner"

[network]
listen_address = "0.0.0.0:8080"
gossip_address = "0.0.0.0:7878"
discovery_nodes = [
    "discovery-1.streamsync.io:7878",
    "discovery-2.streamsync.io:7878"
]

[economics]
stake_account = "YOUR_STAKE_PUBKEY"
reward_address = "YOUR_WALLET_PUBKEY"

[solana]
rpc_url = "https://api.mainnet-beta.solana.com"
```

---

## Running with Docker

```bash
docker run -d \
  --name streamsync \
  -v ./node.toml:/etc/streamsync/node.toml:ro \
  -v ./data:/data \
  -p 8080:8080 \
  -p 7878:7878 \
  streamsync/node:latest
```

---

## Running with systemd

```ini
# /etc/systemd/system/streamsync.service
[Unit]
Description=StreamSync Node
After=network.target

[Service]
Type=simple
User=streamsync
ExecStart=/usr/local/bin/streamsync run --config /etc/streamsync/node.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable streamsync
sudo systemctl start streamsync
```

---

## Verification

```bash
# Check health
curl http://localhost:8080/health

# Check node status
streamsync status

# View logs
journalctl -u streamsync -f
```

---

## Maintenance

### Updates

```bash
# Pull latest
git pull origin main
cargo build --release

# Restart
sudo systemctl restart streamsync
```

### Backup

```bash
# Backup data
tar -czf backup-$(date +%Y%m%d).tar.gz ./data

# Backup config
cp node.toml node.toml.backup
```

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Not receiving queries | Check firewall, verify stake |
| High latency | Check disk I/O, network |
| Memory issues | Reduce cache size |
| Connection refused | Verify ports open |

---

## Next Steps

- [Node Types](node-types.md) - Choose specialization
- [Performance Tuning](performance-tuning.md) - Optimize
- [Monitoring](monitoring.md) - Set up alerts
