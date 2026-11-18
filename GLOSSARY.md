# vai

A programming language whose surface syntax draws on Ruby and Elixir, and whose every construct lowers to idiomatic Rust with Rust's semantics unchanged.

## Language

**vai**:
The language itself; Portuguese imperative of "go".
_Avoid_: Vai (capitalised), vai-lang

**vai source**:
Code written in vai, the input to the compiler.

**Generated Rust**:
The Rust code produced from vai source; meant to be readable and idiomatic, not an opaque intermediate.
_Avoid_: output, IR, target code

**Lowering**:
The mapping of one vai construct onto the Generated Rust it stands for.
_Avoid_: transpiling (for a single construct), compiling

**Sugar**:
A vai construct with no direct Rust spelling that still lowers to plain Rust without changing semantics.

**Crate interop**:
Calling any Rust crate from vai source, and consuming vai code from Rust, with no wrapper layer.
