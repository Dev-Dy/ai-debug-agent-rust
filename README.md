# AI Debugging Agent

A backend system that processes logs asynchronously and returns AI-generated debugging insights.

---

## What this project does

Instead of analyzing logs in the request itself, this system:

* Accepts logs via API
* Pushes them to a Redis Stream
* Processes them in background workers
* Stores job state/result in Redis
* Lets client fetch result using job ID

---

## How it works

```
Client → API → Redis Queue → Worker → AI → Redis (result)
                                      ↓
                               GET /result/:job_id
```

---

## Features

* Async job processing (non-blocking API)
* Redis Streams consumer-group queue
* Worker system using Tokio
* Retry (max 3 attempts)
* Dead Letter Queue (DLQ) for failed jobs
* Concurrency control using semaphore
* Result storage with TTL

---

## Tech stack

* Rust (Axum, Tokio)
* Redis
* OpenAI API
* Docker

---

## API

### POST /analyze

Request:

```json
{
  "logs": "database timeout error"
}
```

Response:

```json
{
  "job_id": "<job_id>",
  "status": "queued"
}
```

---

### GET /result/:job_id

While processing:

```json
{
  "job_id": "<job_id>",
  "status": "running",
  "retry": 0,
  "result": null,
  "error": null,
  "updated_at_ms": 1710000000000
}
```

After completion:

```json
{
  "job_id": "<job_id>",
  "status": "completed",
  "retry": 0,
  "result": "Root cause explanation...",
  "error": null,
  "updated_at_ms": 1710000000000
}
```

---

## Run locally

### Start Redis

```
docker run -d -p 6379:6379 --name redis redis
```

### Run API + workers in one process

```
$env:OPENAI_API_KEY="..."
cargo run -- all
```

### Run API and workers as separate processes

```
cargo run -- api
```

```
$env:OPENAI_API_KEY="..."
cargo run -- worker
```

---

## Project structure

```
src/
 ├── handlers/
 ├── worker/
 ├── queue/
 ├── services/
 ├── models/
 ├── app.rs
 ├── app_state.rs
```

---

## Notes

* Uses Redis Streams with consumer groups and explicit ACK
* Semaphore limits concurrent AI calls
* Retry logic prevents infinite loops
* Failed jobs go to a DLQ stream

---

## What I focused on

* Clean separation (API vs worker vs service)
* Async processing
* Fault tolerance (retry + DLQ)
* Basic distributed system design

---

## Possible improvements

* Replace polling with WebSocket
* Add metrics/logging
* Use Redis streams or Kafka
* Add auth + rate limiting
