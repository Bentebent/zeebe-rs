# 2. Protobuf library

Date: 2025-01-08

## Status

Accepted

## Context

Zeebe uses gRPC as its communication protocol. Therefore we need a way to consume protocol buffers as a basis for our implementation.

## Decision

We have decided to use [tonic](https://github.com/hyperium/tonic) for the gRPC implementation.

## Consequences

Tonic has emerged as the most widely used gRPC implementation in the Rust ecosystem. Allows for easier onboarding due to the maturity of the library, available documentation and examples.
