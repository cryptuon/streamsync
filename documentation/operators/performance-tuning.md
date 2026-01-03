# Performance Tuning

Optimize your node for maximum rewards.

---

## Key Metrics

| Metric | Target | Impact |
|--------|--------|--------|
| **Latency (p99)** | < 10ms | Win rate |
| **Cache Hit Rate** | > 90% | Speed |
| **Uptime** | > 99.9% | Selection |
| **Accuracy** | 100% | Rewards |

---

## Database Tuning

```toml
[database]
memory_limit = "80%"
threads = 0  # Auto-detect
cache_size_mb = 4096
wal_mode = true
```

### DuckDB Optimizations

```sql
-- Increase memory
PRAGMA memory_limit='64GB';

-- Use parallel execution
PRAGMA threads=16;

-- Enable statistics
PRAGMA enable_progress_bar;
```

---

## Cache Optimization

```toml
[performance]
cache_ttl_seconds = 5
max_concurrent_queries = 100

[specialization]
cache_capacity_gb = 32
eviction_policy = "lru"
```

### Hot Data Strategy

1. **Monitor access patterns**
2. **Pre-warm cache** on startup
3. **Tune TTL** based on data freshness needs

---

## Network Optimization

```bash
# Increase socket buffers
sudo sysctl -w net.core.rmem_max=134217728
sudo sysctl -w net.core.wmem_max=134217728

# Enable TCP fast open
sudo sysctl -w net.ipv4.tcp_fastopen=3
```

---

## System Tuning

```bash
# Disable swap
sudo swapoff -a

# Set CPU governor
sudo cpupower frequency-set -g performance

# Increase file limits
ulimit -n 1000000
```

---

## Monitoring

```bash
# Watch key metrics
streamsync metrics watch

# Export to Prometheus
curl localhost:9090/metrics
```

---

## Benchmarking

```bash
# Run local benchmark
streamsync benchmark --queries 10000

# Compare with network
streamsync benchmark --network
```
