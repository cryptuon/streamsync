# Node API

API for node operators and network management.

---

## Base URL

```
http://localhost:8080/node
```

---

## Health & Status

### Health Check

```http
GET /health
```

```json
{
  "status": "healthy",
  "uptime": 86400,
  "version": "1.0.0"
}
```

### Node Status

```http
GET /status
```

```json
{
  "nodeId": "node-001",
  "specialization": "speed-runner",
  "stake": 50000,
  "queriesServed": 125000,
  "winRate": 0.73,
  "avgLatency": 4.2,
  "pendingRewards": 125.5
}
```

---

## Metrics

### Prometheus Metrics

```http
GET /metrics
```

```
# HELP streamsync_queries_total Total queries served
# TYPE streamsync_queries_total counter
streamsync_queries_total{result="success"} 125000
streamsync_queries_total{result="error"} 50

# HELP streamsync_query_latency_ms Query latency
# TYPE streamsync_query_latency_ms histogram
streamsync_query_latency_ms_bucket{le="1"} 50000
streamsync_query_latency_ms_bucket{le="5"} 100000
streamsync_query_latency_ms_bucket{le="10"} 120000
```

---

## Peer Management

### List Peers

```http
GET /peers
```

### Add Peer

```http
POST /peers
Content-Type: application/json

{
  "address": "192.168.1.100:7878"
}
```

---

## Administration

### Graceful Shutdown

```http
POST /admin/shutdown
```

### Reload Configuration

```http
POST /admin/reload
```

---

## WebSocket

### Subscribe to Events

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.send(JSON.stringify({
  type: 'subscribe',
  events: ['query', 'reward', 'peer']
}));

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log(data.type, data.payload);
};
```
