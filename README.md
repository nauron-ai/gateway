# Nauron Gateway

Gateway is the authenticated HTTP API entrypoint for Nauron.

It provides a single public service boundary in front of internal Nauron systems. Its role is to keep access control, request routing, API contracts, and integration with internal processing services outside of individual workers and domain services.

Gateway is responsible for:

| Area | Responsibility |
| --- | --- |
| API boundary | Exposes the external HTTP API. |
| Access control | Authenticates and authorizes requests. |
| Isolation | Keeps internal services away from direct client access. |
| Dispatching | Accepts client requests and dispatches work to internal pipelines. |
| Status | Tracks processing state and exposes read APIs. |
| Artifacts | Returns processed artifacts through controlled endpoints. |

Internal services communicate through PostgreSQL, Redpanda/Kafka, and S3-compatible object storage. Clients should talk to Gateway, not directly to those systems.

## Requirements

- Rust 1.95.0
- PostgreSQL
- Redpanda/Kafka
- S3-compatible object storage

## Configuration

Gateway is configured through environment variables supplied to the container.

Core variables:

| Variable | Description |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection string. |
| `GATEWAY_LISTEN_ADDR` | HTTP listen address, for example `0.0.0.0:8080`. |
| `KAFKA_BROKERS` | Redpanda/Kafka broker list. |
| `S3_ENDPOINT` | S3-compatible object storage endpoint. |
| `S3_ACCESS_KEY` | Object storage access key. |
| `S3_SECRET_KEY` | Object storage secret key. |
| `S3_REGION` | Object storage region. |
| `S3_FORCE_PATH_STYLE` | Enables path-style access for S3-compatible providers. |
| `MIR_INPUT_BUCKET` | Bucket for uploaded source documents. |
| `MIR_OUTPUT_BUCKET` | Bucket for generated artifacts. |

Kafka TLS variables are required only when connecting to TLS-enabled brokers:

| Variable | Description |
| --- | --- |
| `KAFKA_TLS_CA` | CA certificate path. |
| `KAFKA_TLS_CERT` | Client certificate path. |
| `KAFKA_TLS_KEY` | Client private key path. |

Topic names are also configurable through environment variables.

## Container Image

Images are published to GitHub Container Registry:

```bash
docker pull ghcr.io/nauron-ai/gateway:0.1.0
```

The image expects configuration through environment variables.
