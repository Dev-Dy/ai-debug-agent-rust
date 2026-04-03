# AI Debugging Agent

A backend system that processes logs asynchronously and returns AI-generated debugging insights.

---

## What this project does

Instead of analyzing logs in the request itself, this system:

* Accepts logs via API
* Pushes them to a queue (Redis)
* Processes them in background workers
* Stores the result
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
* Redis-based queue
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
  "analysis": "Job queued: <job_id>"
}
```

---

### GET /result/:job_id

While processing:

```json
{
  "status": "processing"
}
```

After completion:

```json
{
  "status": "completed",
  "result": "Root cause explanation..."
}
```

---

## Run locally

### Start Redis

```
docker run -d -p 6379:6379 --name redis redis
```

### Run app

```
cargo run
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

* Uses Redis list as queue (LPUSH + RPOP)
* Semaphore limits concurrent AI calls
* Retry logic prevents infinite loops
* Failed jobs go to DLQ

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
