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

**Block**:
A closure passed to a call with `do |args| … end`, in the Ruby sense. Definition bodies (`def … end`) are not Blocks.
_Avoid_: lambda, do-block (for definition bodies)

**Binding**:
A name introduced by assignment, a parameter or a pattern; immutable unless declared `mut`.
_Avoid_: variable (for immutable bindings)

**Rebinding**:
Assigning to an immutable Binding in the same scope, which introduces a new Binding of that name; it may change type.
_Avoid_: reassignment, shadowing (Rust's term for the lowered form)

**Mutation**:
Assigning to, or calling a mutating method on, a `mut` Binding; the value changes in place.

**Receiver mode**:
How an instance method takes its receiver: borrowed (default), `mut` (mutably borrowed), `own` (by value), or none (`def self.x`, a class method).
_Avoid_: self type, self kind

**Borrow source**:
The parameters, field, or `static` that a returned reference borrows from, written with `from`.
_Avoid_: lifetime (for the vai-level concept)

**Implicit lifetime**:
The single lifetime a struct or enum with reference fields receives without the author naming it.
