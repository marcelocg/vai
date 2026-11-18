# 1. `=` declares, rebinds or mutates depending on the binding

Status: accepted (2026-11-18). Decided in [Bindings and mutability](https://github.com/marcelocg/vai/issues/7).

## Context

vai draws on both Ruby and Elixir, and they disagree on `=`. In Ruby, `=` mutates. In Elixir, `=` rebinds, and in Rust terms that is shadowing. The two only diverge observably when an inner scope assigns to an outer name, as in the accumulator loop `total = total + x` inside a `for`: Ruby gets the sum, Elixir and Rust shadowing get 0. vai must lower to Rust with unchanged semantics and without type information.

## Decision

There is no declaration keyword. `mut x = e` declares a mutable binding.

`x = e`:
- declares `x` when it is unbound;
- **mutates** it when `x` is a `mut` binding;
- **rebinds** it (`let x = e;`) when `x` is immutable and bound in the same scope;
- is a **compile error** when `x` is immutable and bound in an outer scope.

`mut` is never inferred. Mutating methods need a `mut` binding, as in Rust.

## Consequences

- Immutable by default, as in Rust. Elixir-style rebinding, including type-changing rebinding, works within a scope. Ruby-style mutation works where it was declared.
- The silent-shadowing trap becomes a vai diagnostic.
- The same statement `x = e` lowers differently depending on the binding's declaration. Readers must know whether `x` is `mut`, though the `mut` keyword sits on the declaration.
- It is decidable syntactically from scopes, so it doesn't depend on [Syntactic-only lowering vs type-aware compiler](https://github.com/marcelocg/vai/issues/16).

## Alternatives rejected

- **Elixir pure (always rebind):** silently breaks Ruby-style accumulation.
- **Ruby pure (always mutate, infer `let mut`):** erases Rust's immutability signal, and inferring `mut` for method calls needs type information.
- **Explicit `let`:** works, but is noise that both taste references avoid.
