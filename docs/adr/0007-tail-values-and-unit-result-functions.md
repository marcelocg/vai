# 7. Tail values and unit-result functions

Status: accepted (2026-11-24). Decided in [Error handling: Result and Option ergonomics](https://github.com/marcelocg/vai/issues/11).

## Context

vai has no semicolons ([Block and layout style](https://github.com/marcelocg/vai/issues/6)). Rust's split between a tail expression and a statement ending in `;` therefore has to come from somewhere else. Lowering is syntactic (ADR 0006), so it cannot use the type of the last line. `Ok(())` at the end of fallible procedures is a frequent piece of ceremony (pain point #7).

## Decision

- **Declared return type:** the last expression is the Tail value and is lowered without `;`.
- **No declared return type:** the last line is lowered as a statement, with `;`.
- **Closures:** the last expression is always the Tail value.
- **Unit-result function:** the declared return type's last path segment is `Result` and its first generic argument is `()`. Examples: `Result<(), E>`, `io.Result<()>`, `anyhow.Result<()>`.
  - The body lowers like a unit function: every line becomes a statement.
  - `Ok(())` is appended unless the last line is a `return`.
  - A bare `return` lowers to `return Ok(())`.

## Considered options

- **Last expression is the tail unless it ends in `?`:** rejected. A syntactic compiler cannot tell a unit-typed last line (`println(...)`) from a Result-typed one (`f.flush()`).
- **Implicit `Ok` around any tail:** rejected by ADR 0006, because it needs types.

## Consequences

- In a Unit-result function, a last line that returns a `Result` without `?` has its error dropped. Rust only warns about this (`unused_must_use`). The vai diagnostic layer reports that warning as an error on lowered lines.
- A Unit-result function cannot return a `Result` expression as its tail. It writes `return f.flush()` or `f.flush()?`.
- A type alias that hides `()`, such as `type Res = Result<(), E>`, does not trigger the rule, because the rule only looks at syntax.
