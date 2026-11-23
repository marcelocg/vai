# 4. Generics: `<T>` without turbofish, and implicit generic parameters

Status: accepted (2026-11-23). Decided in [Generics, traits and impls syntax](https://github.com/marcelocg/vai/issues/10).

## Context

Pain point #2 includes the turbofish (`::<T>`). Rust requires it in expressions because `a.f<b>(c)` is ambiguous with comparisons. Pain point #6 is `impl<T> X for Y<T> where …` boilerplate. Lowering must stay syntactic.

## Decision

- Generic arguments use `<…>` everywhere. In expressions the parser applies the C# disambiguation rule: `name<types>` directly followed by `(`, `.` or `::` is a generic argument list. vai never emits or requires a turbofish.
- In function signatures and `impl` headers, an undeclared single capital letter (optionally followed by digits) is an implicit generic parameter. Bounds go in `where`. Type definitions declare their parameters explicitly.

## Consequences

- vai keeps Rust's spelling of generics, so docs, crate signatures and remapped diagnostics read the same.
- Parser lookahead is needed for `<`. The one leftover ambiguity (two comparisons passed as call arguments) resolves to generics, and parentheses fix it.
- Rust's single-letter convention for type parameters becomes semantic. A real type named `T` needs `::T` or an explicit declaration.

## Alternatives rejected

- **`[T]` (Scala/Go/Nim):** syntactically resolvable, but it reads like indexing.
- **`{T}`:** clashes with struct literals.
- **Keep turbofish:** it leaves the pain point in place.
- **Implicit parameters on type definitions:** a type's parameter list is its interface, so it should stay explicit.
