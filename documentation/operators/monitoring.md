# Monitoring

Set up monitoring and alerts for your node.

---

## Prometheus Metrics

### Enable Metrics

```toml
[metrics]
enabled = true
address = "0.0.0.0:9090"
```

### Key Metrics

| Metric | Description |
|--------|-------------|
| `streamsync_queries_total` | Total queries served |
| `streamsync_query_latency_ms` | Query latency histogram |
| `streamsync_cache_hit_rate` | Cache hit percentage |
| `streamsync_peer_count` | Connected peers |
| `streamsync_rewards_pending` | Pending rewards |

---

## Grafana Dashboard

```bash
# Import dashboard
curl -X POST http://grafana:3000/api/dashboards/import \
  -H "Content-Type: application/json" \
  -d @grafana-dashboard.json
```

### Dashboard Panels

- Query throughput
- Latency percentiles
- Win rate over time
- Earnings chart
- Peer connections

---

## Alerting

### Example Alerts

```yaml
# alerts.yml
groups:
  - name: streamsync
    rules:
      - alert: HighLatency
        expr: streamsync_query_latency_ms_p99 > 20
        for: 5m

      - alert: LowWinRate
        expr: streamsync_race_win_rate < 0.5
        for: 15m

      - alert: NodeDown
        expr: up{job="streamsync"} == 0
        for: 1m
```

---

## Log Aggregation

```toml
[logging]
level = "info"
format = "json"
file = "./logs/streamsync.log"
```

### With Loki

```yaml
# promtail.yml
scrape_configs:
  - job_name: streamsync
    static_configs:
      - targets: [localhost]
        labels:
          job: streamsync
          __path__: /var/log/streamsync/*.log
```

---

## Health Checks

```bash
# Basic health
curl http://localhost:8080/health

# Detailed status
curl http://localhost:8080/status

# Peer connectivity
curl http://localhost:8080/peers
```

---

## Uptime Monitoring

Use external monitoring services:

- UptimeRobot
- Pingdom
- Better Uptime

Monitor: `http://your-node:8080/health`
