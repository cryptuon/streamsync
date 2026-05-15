# Gossip Protocol

How nodes maintain consistent network state.

---

## Overview

StreamSync uses a gossip protocol for:

- Node discovery
- Health monitoring
- State synchronization
- Failure detection

---

## Protocol Modes

### Push-Pull (Default)

Combines push and pull for best consistency:

```mermaid
sequenceDiagram
    participant A as Node A
    participant B as Node B

    A->>B: Push: My state updates
    B->>A: Pull: Request your full state
    A-->>B: State digest
    B->>B: Merge states
```

### Configuration

```toml
[gossip]
protocol = "push-pull"
fanout = 3
pull_interval_seconds = 5
heartbeat_interval_seconds = 1
failure_threshold_missed_heartbeats = 5
```

---

## Message Types

| Message | Purpose |
|---------|---------|
| `Heartbeat` | Liveness check |
| `Push` | Send state updates |
| `Pull` | Request state |
| `Sync` | Full state sync |
| `Suspect` | Report suspected failure |

---

## Failure Detection

```mermaid
stateDiagram-v2
    [*] --> Alive
    Alive --> Suspect: 3 missed heartbeats
    Suspect --> Alive: Heartbeat received
    Suspect --> Dead: 5 missed heartbeats
    Dead --> Alive: Recovery detected
```

Suspected nodes are:
- Excluded from query routing
- Gossiped to other nodes
- Monitored for recovery

---

## Network Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 7878 | UDP/TCP | Gossip messages |
| 7879 | TCP | State sync |

---

## Tuning

| Parameter | Low Latency | High Consistency |
|-----------|-------------|------------------|
| `fanout` | 2 | 5 |
| `heartbeat_interval` | 500ms | 2s |
| `pull_interval` | 10s | 2s |
