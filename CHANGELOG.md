# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2024-12-31

### Fixed
- Fixed repository URL in Cargo.toml

## [0.1.0] - 2024-12-31

### Added

- Initial release
- Arazzo 1.0.x parser and validator (`arazzo-core`)
- Workflow planner with dependency graph
- Runtime executor with HTTP client (`arazzo-exec`)
- OpenAPI resolution and compilation
- Secrets providers (env, file, AWS, GCP)
- Policy enforcement (SSRF protection, rate limits)
- Retry with exponential backoff
- Postgres persistence (`arazzo-store`)
- CLI with execute, start, resume, cancel commands
- Event streaming and metrics
- Webhook notifications

