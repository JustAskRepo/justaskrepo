## What does this PR do?

<!-- Brief description of the change -->

## Architecture Checklist

Before requesting review, verify ALL of the following:

### Module Boundaries
- [ ] No module imports another module's `domain/`, `application/`, or `infrastructure/` types
- [ ] All inter-module interactions go through `api.rs` Commands and Queries
- [ ] Cross-module state changes use the EventBus, not direct function calls

### Module Structure  
- [ ] Any new module has all four layers: `api.rs`, `domain/`, `application/`, `infrastructure/`
- [ ] `mod.rs` only re-exports from `api.rs`
- [ ] Domain events are in `domain/events.rs` and implement `DomainEvent` trait
- [ ] Event names are past tense (`RepoIndexedEvent`, not `IndexRepoEvent`)

### CQRS API Conventions
- [ ] New write operations are a `<Verb><Noun>Command` with `handle_<verb>_<noun>()` handler
- [ ] New read operations are a `Get<Noun>Query` with `handle_get_<noun>()` handler
- [ ] All new `api.rs` handlers are `async fn`

### Shared Kernel
- [ ] No business logic added to `shared_kernel/`
- [ ] Only new primitive ID types or cross-cutting concerns added to `shared_kernel/`

### ADR
- [ ] If this adds a new module → new ADR created in `backend/ADR/`
- [ ] If this changes a module's public `api.rs` surface → existing ADR updated
- [ ] If this changes an architectural pattern → `backend/ARCHITECTURE.md` updated

### Tests
- [ ] Unit tests for any new domain logic
- [ ] Integration tests for any new `api.rs` handlers

## Architecture Impact

<!-- Does this change any module boundaries, public API surfaces, or event contracts? -->
<!-- If yes, which ADR covers this? -->
