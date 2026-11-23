# 5. `#` comments, `@` attributes, and bang-free macro calls

Status: accepted (2026-11-23). Decided in [Calling Rust macros](https://github.com/marcelocg/vai/issues/12).

## Context

The `macro!` bang is pain point #5, but a syntactic compiler can't tell a macro call from a function call without it. Rust spells attributes `#[…]`, and Ruby and Elixir use `#` for comments.

## Decision

- vai has a fixed list of **known macros** (std/core) called like functions. Third-party macros become bare-callable through `use macro`. `name!(…)` stays as an escape.
- Comments are `#` and doc comments are `##` / `##!`.
- Attributes are `@name(args)` at item level. Inside function bodies, `@name` is a field of self. Attribute arguments pass through verbatim.

## Consequences

- `@` has two meanings, decided by position: an attribute at item level, a self field in function bodies.
- `#` comments make inline `vai! { }` proc macros fragile, because comment text must lex as Rust tokens. This pushes [Compiler pipeline choice](https://github.com/marcelocg/vai/issues/14) toward standalone `.vai` files.
- A crate macro that is called without `use macro` and without `!` lowers as a function call. rustc then reports "expected function, found macro", and the vai diagnostic should suggest `use macro`.
- The list of known macros is part of the language spec and grows deliberately.
