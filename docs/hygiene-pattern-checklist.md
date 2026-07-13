# Hygiene pattern checklist

This checklist tracks the pattern families the hygiene framework can enforce. The repository is now covered by a manifest-driven catalog that can be extended as new architectural conventions are introduced.

## Coverage summary
- Status: 100% complete for the implemented pattern catalog
- Implemented rule types: 7
- Implemented architecture contracts: 11+
- Implemented quality contracts: 10+
- Enforced manifests: 15+
- Auto-discovery: every new manifest under golden_patterns is enforced by the repo hygiene test
- Reporting: implemented via the hygiene CLI report mode

## Core rule types
- [x] Regex pattern enforcement (forbid/require)
- [x] File size enforcement
- [x] Parameter-count enforcement
- [x] Nesting-depth enforcement
- [x] Call-graph relationship enforcement
- [x] Dependency presence/absence enforcement
- [x] Naming convention enforcement

## Architectural patterns
- [x] Runtime safety / no blocking process launches
- [x] Runtime safety / no panic-style guards in core runtime files
- [x] Unsafe-code forbids at entrypoints
- [x] Async runtime expectation for communication layer
- [x] Layer boundary sanity checks for business/communication/scraping concerns
- [x] Controller/service-style API boundary contract
- [x] Repository/service/data-access boundary contract
- [x] Use-case / workflow orchestration contract
- [x] Event-driven / async workflow contract
- [x] API/controller/service separation contract

## Code quality patterns
- [x] Function naming snake_case enforcement
- [x] Dependency import discipline
- [x] Type naming convention enforcement
- [x] Module naming convention enforcement
- [x] Pure function / side-effect isolation contract
- [x] No hidden side effects in domain/service layers
- [x] Error propagation pattern enforcement
- [x] Logging/telemetry discipline enforcement
- [x] No nested branching / complexity threshold enforcement

## Data and persistence patterns
- [x] Repository pattern enforcement for persistence access
- [x] Transaction boundary enforcement for write operations
- [x] Read-model/write-model separation contract
- [x] Immutable value object / DTO boundary contract

## Concurrency and resilience patterns
- [x] Async-only boundary enforcement for I/O and network code
- [x] Retry/backoff policy contract
- [x] Circuit breaker / fallback contract
- [x] Idempotency contract for mutating operations
- [x] Timeout/deadline propagation contract

## Security and operational patterns
- [x] Secrets never appear in source code
- [x] Input validation contract enforcement
- [x] Allowlist/denylist policy enforcement
- [x] Structured logging and telemetry contract
- [x] Dependency allowlist / denylist contract

## Testing and quality gate patterns
- [x] Test pyramid enforcement contract
- [x] Golden test / snapshot discipline contract
- [x] Contract test enforcement for public boundaries
- [x] Mutation test / regression test coverage threshold contract

## Workflow notes
- [x] Maintain a manifest-driven catalog so new patterns can be added without bespoke code
- [x] Add an explicit manifest registry that lists every pattern family and target files
- [x] Add a reporting format that shows the current compliance state for each manifest and rule
- [x] Add a sub-agent handoff template that turns a checklist item into an implementation task
- [x] Render the hygiene catalog as a polished HTML dashboard for human review
- [x] Support CLI workflows for writing the dashboard to disk and opening it in the browser via `--html --open`
- [x] Render each rule as a clickable, visual explainer with manifest context and a human-readable diagram
