# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build the project
cargo build

# Build with optimizations for release
cargo build --release

# Run tests
cargo test

# Run the server
./target/release/pmetrics server --server-type http --port 1337
```

## Database Setup

The application requires PostgreSQL:

```bash
# Start PostgreSQL using Docker Compose
docker-compose up -d pg

# Initialize database schema and test data
./start-db.sh
```

## Environment Variables

Required PostgreSQL connection environment variables:

```bash
export PGPASSWORD=aargh  # Default dev password
export PGHOST=localhost  # Or 'pg' when using docker-compose
export PGUSER=postgres
export PGDATABASE=postgres
export PGPORT=5432
```

## Testing the API

```bash
# Run test requests against the API
./curltest.sh

# For manual testing with curl
KEY=a-wiWimWyilf  # Test API key
curl -H "X-PMETRICS-API-KEY: $KEY" localhost:1337/api/v1/event
```

## Docker and Deployment

```bash
# Build Docker image
docker build -t pmetrics:latest .

# Run with Docker Compose
docker-compose up

# Deploy with Helm to Kubernetes
cd pmetrics
helm upgrade --namespace pmetrics --install pmetrics . -f values.yaml
```

## Architecture

**pmetrics** is a straightforward metrics and event tracking system built in Rust. It focuses on two main data types:

1. Named numerical measurements with key-value metadata
2. Named events with key-value metadata

The system architecture includes:

- **Storage**: PostgreSQL database with JSONB for flexible key-value metadata
- **API**: REST API built with the Nickel framework
- **Authentication**: API key-based authentication with tenant isolation
- **Clients**: Python client library

The design philosophy emphasizes simplicity and reliability over "Web Scale" distributed systems. It's intended for smaller software teams that need operational metrics without complex infrastructure.

## Database Schema

The database schema consists of three main tables:
- `monitoring.tenant`: Stores tenant information and API keys
- `monitoring.measure`: Stores numerical measurements with metadata
- `monitoring.event`: Stores events with metadata

## API Endpoints

- `GET /api/v1/event`: List recent events
- `POST /api/v1/event`: Create a new event
- `GET /api/v1/measure`: List recent measurements
- `POST /api/v1/measure`: Create a new measurement

All API requests require an `X-PMETRICS-API-KEY` header with a valid API key.

## Codebase Organization

- `src/`: Rust source code
  - `lib.rs`: Library exports
  - `db.rs`: Database connection and operations
  - `audit.rs`: Logging and audit trail functionality
- `schema/`: SQL schema definitions
- `clients/`: Client libraries (Python)
- `pmetrics/`: Helm chart for Kubernetes deployment