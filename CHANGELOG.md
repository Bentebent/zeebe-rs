# Changelog

All notable changes to zeebe-rs will be documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/).

## Unreleased

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## 0.2.0 - 2024-01-28

### Added

- Added new error type `WorkerError<R>`
- Added support for worker handlers that return `Result<T, WorkerError<R>>`
  - Any worker handler that returns a `Result` instead of `()` now automatically reports success/failure to Zeebe
    using `fail_job`, `throw_error` or `complete_job`.

### Changed

- Refactored worker builder for easier re-use of shared configurations
- Updated documentation across solution
- Updated `pizza` example to reflect new worker functionality

## 0.1.0 - 2024-01-27

### Added

- Initial release
- Full support for all Zeebe client requests defined by protobuf
- Async worker support

## Notes

This version is compatible with Zeebe v5.29.2. API should be considered unstable until 1.0.0
