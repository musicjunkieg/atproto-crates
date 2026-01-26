# atproto-xrpcs-helloworld

Example XRPC service with DID:web identity and JWT authentication.

## Overview

Complete AT Protocol XRPC service demonstrating DID:web identity, service document generation, well-known endpoints, and JWT-based authentication patterns.

## Features

- **Complete XRPC service**: Production-ready service implementation with authentication
- **DID:web identity**: Automatic service document generation with well-known endpoint support
- **JWT authorization**: Optional JWT-based authentication with DID document verification
- **Discovery endpoints**: Standard AT Protocol discovery endpoints for service metadata
- **Example patterns**: Reference implementation for building AT Protocol services

## CLI Tools

The following service binary is available:

- **`atproto-xrpcs-helloworld`**: Complete XRPC service demonstrating AT Protocol patterns with DID:web identity and optional JWT authentication

## Usage

The service provides these endpoints:

- `GET /` - HTML hello world page
- `GET /.well-known/did.json` - DID web document
- `GET /.well-known/atproto-did` - AT Protocol DID discovery
- `GET /xrpc/garden.lexicon.ngerakines.helloworld.Hello` - Example XRPC endpoint

### Environment Variables

```bash
# Required
export EXTERNAL_BASE=your-service.com
export SERVICE_KEY=did:key:zQ3sh...

# Optional
export PLC_HOSTNAME=plc.directory
export DNS_NAMESERVERS=8.8.8.8;1.1.1.1
export USER_AGENT="my-xrpc-service/1.0"
```

### Example Requests

```bash
# Unauthenticated request
curl "http://localhost:8080/xrpc/garden.lexicon.ngerakines.helloworld.Hello?subject=World"
# Response: {"message": "Hello, World!"}

# Authenticated request
curl -H "Authorization: Bearer <jwt-token>" \
     "http://localhost:8080/xrpc/garden.lexicon.ngerakines.helloworld.Hello?subject=Alice"
# Response: {"message": "Hello, authenticated Alice!"}
```

## Command Line Examples

```bash
# Start the service
cargo run --bin atproto-xrpcs-helloworld

# Test endpoints
curl http://localhost:8080/
curl http://localhost:8080/.well-known/did.json
curl "http://localhost:8080/xrpc/garden.lexicon.ngerakines.helloworld.Hello?subject=Test"
```

## Use Cases

This example service is ideal for:

- Learning AT Protocol XRPC service patterns
- Testing AT Protocol clients against a known service
- Starting point for building production services
- Understanding DID web document structure

## License

MIT License