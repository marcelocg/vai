# 3. Dot paths resolved by scope and naming convention

Status: accepted (2026-11-23). Decided in [Paths, modules and visibility](https://github.com/marcelocg/vai/issues/9).

## Context

Rust separates paths (`a::b`) from field and method access (`a.b`). `::` is pain point #2. Ruby uses `::` too; only Elixir reads like `Mod.fun`. vai has no type information ([Syntactic-only lowering vs type-aware compiler](https://github.com/marcelocg/vai/issues/16) is still open), so a merged `.` has to be resolved from syntax alone.

## Decision

vai uses one `.`. The head segment decides what the `.` lowers to:
- a local binding, parameter, `self` or `@field` is a value;
- `crate`, `super` and `Self` are path keywords;
- `UpperCamel` and primitive type names are types;
- `SCREAMING_CASE` names are consts, so they are values;
- other `snake_case` names are modules;
- the result of a call or an index is always a value.

`::` stays accepted as an explicit path escape.

## Consequences

- There is no `::` in normal code, and resolution needs only scope tracking, which [Bindings and mutability](https://github.com/marcelocg/vai/issues/7) already requires.
- Rust's naming conventions become **semantic** in vai. Crates that break them, or a local that shadows a module name, need `::`. The diagnostic for a wrong guess has to say so.
- Rust's naming lints (`non_snake_case`, `non_camel_case_types`) effectively become errors, because a name with the wrong case gets resolved differently.

## Alternatives rejected

- **Keep `::`:** it leaves pain point #2 in place.
- **Type-aware resolution:** it would make the compiler read crate metadata just to print paths.
