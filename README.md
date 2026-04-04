# AI Debug Agent

A backend system that asynchronously analyzes application logs and returns AI-generated debugging insights — built in Rust with Axum, Redis Streams, and OpenAI.

---

## Why I built this

Most "AI + API" projects are synchronous — you send a request, wait for OpenAI to respond, get a result. That breaks under load. I wanted to build something closer to how real systems work: a non-blocking queue-based architecture where the API layer and the AI processing layer are fully decoupled.

This project forced me to deal with real distributed systems problems in Rust — consumer group coordination, retry logic with backoff, dead letter queues, and semaphore-controlled concurrency — all in async Rust with Tokio.

---

## Architecture

```
Client → POST /analyze → API Server → Redis Stream
                                           ↓
                                      Worker Pool
                                           ↓
                                      OpenAI API
                                           ↓
                                    Redis (result + TTL)
                                           ↑
                            Client → GET /result/:job_id
```

The API and worker run in the same binary (`cargo run -- all`) or as separate processes for horizontal scaling.

---

## Features

- Non-blocking API — responds immediately with a `job_id`, processes in background
- Redis Streams with consumer groups and explicit ACK
- Worker pool with Tokio — concurrent AI calls with semaphore limits
- Retry logic — up to 3 attempts per job before moving to DLQ
- Dead Letter Queue — failed jobs captured for inspection
- Result storage with TTL — results expire automatically
- Structured logging with `tracing`
- Docker support — single `docker-compose up` spins up Redis + app

---

## Tech stack

| Layer | Tech |
|---|---|
| Web framework | Axum 0.7 |
| Async runtime | Tokio |
| Queue | Redis Streams |
| AI | OpenAI API via reqwest |
| Error handling | thiserror |
| Logging | tracing + tracing-subscriber |
| Containerization | Docker + docker-compose |

---

## API

### `POST /analyze`

Submit logs for analysis. Returns a job ID immediately.

**Request:**
```json
{
  "logs": "ERROR: connection pool timeout after 30s, retries exhausted"
}
```

**Response:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "queued"
}
```

---

### `GET /result/:job_id`

Poll for the result using the job ID.

**While processing:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "retry": 0,
  "result": null,
  "error": null,
  "updated_at_ms": 1710000000000
}
```

**On completion:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "retry": 0,
  "result": "Root cause: connection pool exhaustion. The database is likely under high load or connections are not being released properly. Check for unclosed transactions and consider increasing pool size or adding connection timeout handling.",
  "error": null,
  "updated_at_ms": 1710000000000
}
```

---

## Run locally

### Prerequisites

- Rust (stable)
- Docker

### Start Redis

```bash
docker run -d -p 6379:6379 --name redis redis
```

### Run everything in one process

**Linux / macOS:**
```bash
OPENAI_API_KEY="sk-..." cargo run -- all
```

**Windows (PowerShell):**
```powershell
$env:OPENAI_API_KEY="sk-..."
cargo run -- all
```

### Run API and worker as separate processes

```bash
# Terminal 1 — API server
cargo run -- api

# Terminal 2 — Worker
OPENAI_API_KEY="sk-..." cargo run -- worker
```

### Or use Docker Compose

```bash
OPENAI_API_KEY="sk-..." docker-compose up
```

---

## Project structure

```
src/
├── handlers/       # Axum route handlers (analyze, result)
├── worker/         # Background job processor
├── queue/          # Redis Streams producer/consumer logic
├── services/       # OpenAI API integration
├── models/         # Shared data types
├── app.rs          # Router setup
└── app_state.rs    # Shared state (Redis pool, semaphore)
```

---

## What I learned

**Async traits in Rust are tricky.** `async fn` in traits isn't object-safe by default, which forced me to think carefully about where to use `dyn Trait` vs generics. I ended up using `async-trait` in a few places but tried to minimize it.

**The borrow checker pushes you toward better architecture.** Sharing the Redis connection pool across the API and worker threads required `Arc<RedisPool>` in `AppState`. The compiler forced the ownership question early, which led to a cleaner design than I'd have written in a dynamic language.

**Redis Streams are more complex than a simple queue.** Managing consumer groups, ACK logic, and pending entry lists properly takes more thought than using a basic `LPUSH`/`RPOP`. The DLQ implementation required understanding how Redis tracks unacknowledged messages.

---

## Possible improvements

- Replace polling (`GET /result/:job_id`) with WebSocket for push-based updates
- Add Prometheus metrics endpoint
- Add authentication + rate limiting on the API
- Swap OpenAI for a local model via Ollama for cost control
- Load test the semaphore limits under concurrent traffic

---

## License

MIT
