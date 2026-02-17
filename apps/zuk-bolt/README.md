# zuk-bolt

Stateless sender service for ZukLink distributed streaming platform.

## Overview

`zuk-bolt` is an HTTP service that ingests data and stores it in S3. It follows the **"Flat Storage"** pattern: writes files with UUID-based names without any coordination with receivers.

## Architecture

```
┌─────────────────────┐
│   HTTP Request      │
│   POST /ingest      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  Ingestion Handler  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ IngestionService    │
│ (Domain Logic)      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ S3StorageRepository │
│ (Infrastructure)    │
└──────────┬──────────┘
           │
           ▼
        AWS S3
```

## Project Structure

```
src/
├── main.rs              # Application entry point
├── dto/                 # Data Transfer Objects
│   ├── mod.rs
│   └── ingestion.rs     # Request/Response DTOs
├── handlers/            # Request handlers
│   ├── mod.rs
│   └── ingestion.rs     # Ingestion logic
└── routes/              # HTTP routes
    ├── mod.rs
    └── ingestion.rs     # Ingestion routes
```

## Configuration

Create a `.env` file based on `.env.example`:

```bash
# AWS Configuration for MinIO (local development)
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin123
AWS_ENDPOINT_URL=http://localhost:9000

# S3 Bucket (created by docker-compose)
ZUKLINK_BUCKET=zuklink

# Server
BOLT_HOST=0.0.0.0
BOLT_PORT=3000

# Logging
RUST_LOG=info

# Production AWS (uncomment for real AWS S3)
# AWS_REGION=us-east-1
# AWS_ACCESS_KEY_ID=your-access-key
# AWS_SECRET_ACCESS_KEY=your-secret-key
# ZUKLINK_BUCKET=zuklink-production
# Comment out AWS_ENDPOINT_URL for production
```

## Running

### Local Development (MinIO)

```bash
# Start MinIO (automatically creates 'zuklink' bucket)
docker compose up -d

# Run service
cargo run -p zuk-bolt
```

### Production

```bash
# Set environment variables
export AWS_REGION=us-east-1
export ZUKLINK_BUCKET=zuklink-production

# Run (IAM role for credentials)
cargo run -p zuk-bolt --release
```

## API Endpoints

### Swagger UI

Interactive API documentation is available at:

```
http://localhost:3000/swagger-ui
```

This provides a web interface to explore and test all endpoints.

### Health Check

```bash
GET /health
```

**Response:**
```
OK
```

### Ingest Data

```bash
POST /ingest
Content-Type: application/json

{
  "data": [1, 2, 3, 4, 5]
}
```

**Success Response (201):**
```json
{
  "segment_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Segment ingested successfully"
}
```

**Error Response (400/413/500):**
```json
{
  "error": "Segment size (1000000000 bytes) exceeds maximum (104857600 bytes)"
}
```

## Testing

### Using Swagger UI (Recommended)

1. Open http://localhost:3000/swagger-ui in your browser
2. Explore the available endpoints
3. Click "Try it out" on any endpoint
4. Fill in the request body and click "Execute"

### Using cURL

```bash
# Health check
curl http://localhost:3000/health

# Ingest data
curl -X POST http://localhost:3000/ingest \
  -H "Content-Type: application/json" \
  -d '{"data": [72, 101, 108, 108, 111]}'
```

## Storage Format

Files are stored in S3 with flat structure:

```
s3://zuklink/
├── 550e8400-e29b-41d4-a716-446655440000.zuk
├── 6ba7b810-9dad-11d1-80b4-00c04fd430c8.zuk
└── 7c9e6679-7425-40de-944b-e07fc1f90ae7.zuk
```

- **Format:** `{segment_id}.zuk`
- **Naming:** UUID v4 for uniqueness and sharding

## Error Handling

| Error | Status Code | Description |
|-------|-------------|-------------|
| EmptySegment | 400 | Data cannot be empty |
| SegmentTooLarge | 413 | Exceeds max size (100MB) |
| InvalidData | 400 | Data validation failed |
| StorageFailure | 500 | S3 operation failed |
| SegmentAlreadyExists | 409 | Duplicate segment ID |
| ConfigError | 500 | Configuration error |
| InternalError | 500 | Unexpected error |

## Logging

Structured logs with `tracing`:

```
INFO zuk_bolt: Starting ZukBolt sender service
INFO zuk_bolt: Initializing S3 storage repository bucket="zuklink"
INFO zuk_bolt: Starting HTTP server addr="0.0.0.0:3000"
INFO ingestion: Received ingest request data_size=5
INFO ingestion: Successfully ingested segment segment_id="550e8400-..."
```

Set log level via `RUST_LOG` environment variable.