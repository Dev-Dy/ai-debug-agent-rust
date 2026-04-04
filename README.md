# 🚀 AI Debug Agent (Rust)

A **production-style distributed debugging system** built in Rust that processes logs asynchronously and uses AI to analyze failures at scale.

---

## 🧠 Problem

Debugging backend systems is slow, manual, and reactive:

* Logs are scattered
* Failures are hard to trace
* Engineers spend hours diagnosing issues

---

## 💡 Solution

This project introduces an **AI-powered debugging agent** that:

* accepts logs via API
* processes them asynchronously
* analyzes issues using an external AI service
* scales using worker-based architecture

---

## 🏗️ Architecture

```text
Client → API (Axum) → Redis → Worker Pool → AI API → Result Store
```

* **API Layer** → receives jobs
* **Queue (Redis)** → decouples ingestion & processing
* **Workers** → process jobs concurrently
* **AI Service** → analyzes logs
* **Result Store** → stores processed output

---

## ⚙️ Features

* ⚡ Async job processing using Tokio
* 📦 Redis-backed queue system
* 🔁 Retry + Dead Letter Queue (DLQ)
* 🧵 Worker pool with concurrency control
* 📊 Structured logging using tracing
* 🐳 Docker-based deployment
* 🔌 External AI API integration

---

## 🚀 Quick Start

### 1. Clone the repo

```bash
git clone https://github.com/Dev-Dy/ai-debug-agent-rust.git
cd ai-debug-agent-rust
```

---

### 2. Start system

```bash
docker compose up --build
```

---

### 3. Submit a job

```bash
curl -X POST http://localhost:3000/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "id": "job-1",
    "retry": 0,
    "logs": "error: database connection failed"
  }'
```

---

### 4. Fetch result

```bash
curl http://localhost:3000/results/job-1
```

---

## 📊 Example Flow

1. Client submits logs
2. API pushes job to Redis
3. Worker picks job
4. AI analyzes logs
5. Result stored and returned

---

## 🔥 Why Rust?

* Memory safety without GC
* High-performance async execution
* Strong concurrency guarantees
* Ideal for distributed systems

---

## ⚠️ Current Limitations

* No idempotency (duplicate jobs possible)
* Basic retry logic (no backoff yet)
* No metrics/observability (planned)

---

## 🗺️ Roadmap

* [ ] Redis Streams + consumer groups
* [ ] Job recovery (XCLAIM)
* [ ] Prometheus metrics + tracing
* [ ] Idempotency layer
* [ ] Rate limiting / backpressure
* [ ] Multi-tenant support

---

## 🧪 Tech Stack

* Rust (Tokio async runtime)
* Axum (API framework)
* Redis (queue + storage)
* Reqwest (HTTP client)
* Tracing (logging)
* Docker (deployment)

---

## 🤝 Contributing

Contributions, feedback, and ideas are welcome.

If you find issues or want to improve the system:

* open an issue
* submit a PR

---

## 📌 Use Cases

* Backend debugging automation
* Log analysis pipelines
* Async job processing systems
* AI-assisted observability

---

## ⭐ If You Found This Useful

Give it a star ⭐ and share your feedback!

---

## 👨‍💻 Author

Built as part of a journey to design **production-grade distributed systems in Rust**.
