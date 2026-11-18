# 2. Implicit lifetimes on reference-holding types, and the `from` clause

Status: accepted (2026-11-18). Decided in [References, borrowing and lifetimes syntax](https://github.com/marcelocg/vai/issues/8).

## Context

Lifetime clutter is the #1 pain point. Research ([Where Rust forces explicit lifetimes](https://github.com/marcelocg/vai/issues/5)) showed that forced lifetimes cluster in two places:
- **Types holding references.** The lifetime spreads virally to every containing type.
- **Returned references whose source elision cannot pick.**

Rust considered hiding struct lifetimes ("in-band lifetimes", RFC 2115) and rejected the idea, because a reader could no longer see that a type borrows. vai can't infer borrow sources from function bodies without changing semantics.

## Decision

- A struct or enum with unannotated reference fields gets **one implicit lifetime** shared by all of them.
- A signature that returns a borrowed value states its source with **`from`**: parameters, `self`, `static`, or `@field`. "`@field`" means the enclosing type's implicit lifetime.
- Lowering turns each `from` into a single named lifetime on the outermost references. It is syntactic.
- Anything beyond this uses Rust's `'a` syntax unchanged.

## Consequences

- RFC 2115's concern is answered by keeping the borrow visible elsewhere: the `&` in the field declaration, and a mandatory `from` on every function returning a borrowing value.
- Types with two or more distinct lifetimes are rare, but they fall back to explicit `'a` and lose the sugar.
- `from` gives one shared lifetime to all named sources, which is the `longest<'a>` pattern. Signatures that need distinct output lifetimes need the escape.
