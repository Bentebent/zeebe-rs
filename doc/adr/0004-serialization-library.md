# 4. Serialization library

Date: 2025-01-26

## Status

Accepted

## Context

To be able to build a type safe client that interacts with Zeebe the client needs to be able to serialize/deserialize JSON data.

## Decision

The decision has been made to use [Serde](https://github.com/serde-rs/serde).

## Consequences

Serde is well established and supported implementation for serialization and deserialization of JSON data. Allows for easier onboarding due to the maturity of the library, available documentation and examples.
