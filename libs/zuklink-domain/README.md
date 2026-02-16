# ZukLink Domain Layer

Pure business logic and domain models for the ZukLink distributed streaming platform.

## Overview

This crate implements the **Domain Layer** following **Hexagonal Architecture** (Ports & Adapters) principles. It contains:

- **Entities**: Core domain models (`Segment`, `SegmentId`)
- **Ports**: Trait definitions for external dependencies (`StorageRepository`)
- **Services**: Business logic orchestration (`IngestionService`)
- **Domain Errors**: Business-level error types (`IngestionError`)

## Philosophy

### Dependency Inversion

The domain layer **defines what it needs** via traits (ports), and the infrastructure layer **provides implementations** (adapters). This creates a clean separation:

```
Domain (this crate)     ←─── no dependencies on ───┐
    ↓ defines ports                                 │
Infrastructure          ←─── implements ports ──────┘
    (zuklink-s3)
```

### Zero Infrastructure Dependencies

This crate has **NO** dependencies on:
- ❌ AWS SDK
- ❌ HTTP frameworks
- ❌ Database clients
- ❌ Any I/O libraries

It only depends on:
- ✅ `serde` (serialization)
- ✅ `uuid` (identifiers)
- ✅ `chrono` (timestamps)
- ✅ `thiserror` (error handling)

## Architecture

### Entities

Core domain models that represent business concepts:

```rust
use zuklink_domain::ingestion::{Segment, SegmentId};

// Create a new segment
let segment = Segment::new(vec![1, 2, 3, 4]);
println!("Segment ID: {}", segment.id());
println!("Size: {} bytes", segment.size());
```

### Ports (Traits)

Contracts that infrastructure must implement:

```rust
pub trait StorageRepository: Send + Sync {
    fn save(&self, segment: &Segment, data: &[u8])
        -> impl Future<Output = Result<String, IngestionError>> + Send;
    
    fn get(&self, segment_id: &SegmentId)
        -> impl Future<Output = Result<Vec<u8>, IngestionError>> + Send;
    
    fn exists(&self, segment_id: &SegmentId)
        -> impl Future<Output = Result<bool, IngestionError>> + Send;
    
    fn delete(&self, segment_id: &SegmentId)
        -> impl Future<Output = Result<(), IngestionError>> + Send;
}
```

### Services

Business logic orchestration:

```rust
use zuklink_domain::ingestion::{IngestionService, IngestionConfig};
use zuklink_domain::ports::StorageRepository;

// Service is generic over any StorageRepository implementation
fn create_service<R: StorageRepository>(repository: R) -> IngestionService<R> {
    let config = IngestionConfig::default();
    IngestionService::new(repository, config)
}

// Use the service
async fn ingest<R: StorageRepository>(service: &IngestionService<R>) {
    let data = vec![1, 2, 3, 4, 5];
    let segment_id = service.ingest_data(data).await.unwrap();
    println!("Ingested: {}", segment_id);
}
```

## Static Dispatch (No async_trait)

We use **native Rust async traits** with `impl Future` return types instead of `async_trait` for:

- ✅ **Zero-cost abstractions**: No heap allocations
- ✅ **Static dispatch**: Compiler monomorphization
- ✅ **Better performance**: No dynamic dispatch overhead

### Example: Implementing the Port

```rust
use zuklink_domain::ports::StorageRepository;
use zuklink_domain::ingestion::{Segment, SegmentId, IngestionError};
use std::future::Future;

struct MyStorage;

impl StorageRepository for MyStorage {
    fn save(&self, segment: &Segment, data: &[u8])
        -> impl Future<Output = Result<String, IngestionError>> + Send
    {
        async move {
            // Your implementation here
            let key = format!("data/{}.zuk", segment.id());
            // ... store to S3, filesystem, etc.
            Ok(key)
        }
    }

    // Implement other methods...
    fn get(&self, segment_id: &SegmentId)
        -> impl Future<Output = Result<Vec<u8>, IngestionError>> + Send
    {
        async move {
            // Your implementation
            Ok(vec![])
        }
    }

    fn exists(&self, segment_id: &SegmentId)
        -> impl Future<Output = Result<bool, IngestionError>> + Send
    {
        async move { Ok(false) }
    }

    fn delete(&self, segment_id: &SegmentId)
        -> impl Future<Output = Result<(), IngestionError>> + Send
    {
        async move { Ok(()) }
    }
}
```

## Business Rules

The `IngestionService` enforces business invariants:

### 1. No Empty Segments
```rust
let result = service.ingest_data(vec![]).await;
assert!(matches!(result, Err(IngestionError::EmptySegment)));
```

### 2. Maximum Segment Size
```rust
let config = IngestionConfig {
    max_segment_size: 100 * 1024 * 1024, // 100MB
    min_segment_size: 1,
};
```

### 3. Immutability
Once created, segments never change. The `storage_key` can only be set once after persistence.

## Error Handling

Domain errors abstract infrastructure failures:

```rust
use zuklink_domain::ingestion::IngestionError;

match service.ingest_data(data).await {
    Ok(segment_id) => println!("Success: {}", segment_id),
    Err(IngestionError::EmptySegment) => println!("Cannot ingest empty data"),
    Err(IngestionError::SegmentTooLarge { size, max }) => {
        println!("Segment too large: {} > {}", size, max)
    },
    Err(IngestionError::StorageFailure(msg)) => {
        println!("Storage error: {}", msg)
    },
    Err(e) => println!("Other error: {}", e),
}
```

## Testing

The domain layer includes comprehensive tests:

```bash
# Run all tests
cargo test -p zuklink-domain

# Run with output
cargo test -p zuklink-domain -- --nocapture

# Check for issues
cargo clippy -p zuklink-domain -- -D warnings
```

### Test Coverage

- ✅ Entity creation and validation (6 tests)
- ✅ Error types and messages (4 tests)
- ✅ Service business logic (7 tests)
- ✅ Mock repository implementation (5 tests)

## Integration Example

Here's how the domain layer integrates with infrastructure:

```rust
// 1. Infrastructure implements the port
use zuklink_s3::S3StorageRepository;

// 2. Domain service uses the implementation
use zuklink_domain::ingestion::IngestionService;

async fn setup() {
    // Infrastructure layer
    let s3_client = /* AWS S3 client */;
    let repository = S3StorageRepository::new(s3_client, "my-bucket".into());
    
    // Domain layer (generic over repository)
    let service = IngestionService::with_repository(repository);
    
    // Use the service
    let data = vec![1, 2, 3];
    let segment_id = service.ingest_data(data).await.unwrap();
}
```

## API Reference

### Core Types

- `Segment` - Immutable data chunk
- `SegmentId` - Unique segment identifier (UUID v4)
- `IngestionService<R>` - Business logic orchestration
- `IngestionConfig` - Service configuration
- `IngestionError` - Domain errors

### Traits

- `StorageRepository` - Storage backend contract

### Methods

#### IngestionService

- `new(repository, config)` - Create with custom config
- `with_repository(repository)` - Create with default config
- `ingest_data(data)` - Main ingestion method
- `get_segment_data(segment_id)` - Retrieve segment
- `segment_exists(segment_id)` - Check existence
- `delete_segment(segment_id)` - Delete segment

## Design Decisions

### Why No async_trait?

`async_trait` uses dynamic dispatch (`Box<dyn Future>`) which:
- Allocates on the heap
- Prevents compiler optimizations
- Adds runtime overhead

Native `impl Future` allows the compiler to inline and optimize aggressively.

### Why Separate Error Types?

Domain errors (`IngestionError`) are different from infrastructure errors (e.g., `aws_sdk_s3::Error`). This:
- Decouples domain from infrastructure
- Makes testing easier
- Provides cleaner API for consumers

### Why No Data in Segment Entity?

The `Segment` entity doesn't store the actual bytes to:
- Keep entities lightweight
- Avoid memory overhead
- Separate metadata from payload

The service passes data directly to the repository.

## Contributing

When adding new features:

1. **Entities**: Add to `src/ingestion/entity.rs`
2. **Business Logic**: Add to `src/ingestion/service.rs`
3. **Ports**: Add to `src/ports.rs`
4. **Errors**: Add to `src/ingestion/error.rs`
5. **Tests**: Include unit tests for all business rules

## License

Apache-2.0

## Related Crates

- `zuklink-s3` - S3 adapter implementation
- `zuklink-yellowpage` - Cluster coordination
- `zuk-bolt` - Sender application
- `zuk-sink` - Receiver application