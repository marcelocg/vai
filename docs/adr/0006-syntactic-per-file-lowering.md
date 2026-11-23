# 6. Lowering is syntactic and per-file

Status: accepted (2026-11-23). Decided in [Syntactic-only lowering vs type-aware compiler](https://github.com/marcelocg/vai/issues/16).

## Context

Some attractive Sugar needs type information: automatic `&mut`, paren-less calls, implicit `Ok`. Getting types without reimplementing rustc means reading crate metadata through rustdoc JSON (unstable) or sysroot inspection, as Incan and Galvan do, and that comes with a large complexity and maintenance cost ([Prior art: languages layered over Rust](https://github.com/marcelocg/vai/issues/2)). ADRs 0001–0005 were already designed to be decidable from syntax.

## Decision

- Lowering depends only on the contents of the `.vai` file being lowered: scopes, declarations, `use` / `use macro`, and naming case.
- Lowering never uses types, crate metadata, or other `.vai` files.
- Types may inform diagnostics only.

## Consequences

- Generated Rust can be predicted from the vai source alone. Files lower in parallel and cache individually.
- The compiler needs no dependency on rustdoc JSON or rustc internals, which fits a stable-toolchain pipeline.
- Sugar that needs types is permanently out: automatic `&`/`&mut`, paren-less zero-argument calls, inferred `mut`, automatic clone or borrow, and implicit `Ok` around non-unit tails.
- Cross-file knowledge, if ever needed, requires a new ADR.
