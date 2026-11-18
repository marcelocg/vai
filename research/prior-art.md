# Prior art: languages layered over Rust

Ticket: [#2](https://github.com/marcelocg/vai/issues/2). Researched 2026-11-18 against project repos and first-party docs. Repo activity dates come from the GitHub API (`pushed_at`) on that day.

## Question

Which languages compile to Rust, or put a different surface over Rust semantics? For each one: what it keeps and what it breaks of Rust semantics, how it handles crate interop, how diagnostics reach the user, what its pipeline looks like, and whether it is still alive. Which lessons apply to vai?

## Short answer

- **No existing language does what vai plans to do.** None of them keeps Rust semantics 1:1, uses a Ruby or Elixir surface, and promises interop in both directions.
- **Every source-to-Rust language with traction changes the ownership model to hide it.** Incan plans clones and borrows behind the user's back. Galvan and Ruchy add copy-on-write or `Rc`/`Arc` reference types. Fable uses ref-counted pointers.
- **The well-known names don't compile to Rust.**
  - Borgo and Lisette have a Rust-like syntax but compile to Go.
  - Dada is a Rust-like language that compiles to WebAssembly.
  - Rune, Mun, Rhai and Gluon are languages you embed in a Rust host.
- **Mapping rustc diagnostics back to source is unsolved in Rust itself.** rustc has no `#line` or source-map input. Several pre-RFCs (2017, 2023, 2024) ended without a decision.
- **The working approach is to type-check in the front end and hide rustc.** Incan reports errors from its own checker and treats rustc failures on generated code as compiler bugs. vai cannot do this: it keeps Rust's borrow checker, so rustc will report real user errors on Generated Rust. That makes span mapping a core part of vai's design, not a nice-to-have.

## Survey

### A. Source-to-source languages that emit Rust

| Language | Surface | Kept of Rust | Broke of Rust | Crate interop | Diagnostics | Pipeline | Alive? |
|---|---|---|---|---|---|---|---|
| **Incan** ([repo](https://github.com/encero-systems/incan), [docs](https://incan.io)) | Python-like, indentation | `Result`/`Option`/`?`, traits, explicit `mut`, native build | Value semantics. The backend "duckborrowing" planner decides move, borrow, `&mut`, `.clone()` or `.into()` per use site. No lifetime syntax. Explicit ownership annotations are only planned (RFC 164) | `import rust::crate`, `from rust::std::collections import HashMap`, with version and feature annotations on the import. Its own toolchain ("Oven", direct `rustc`, Cargo not used by normal commands). Rust consumers can use generated library crates as Cargo path deps; a curated Rust-hosted API is future work (RFC 097) | Its own type checker with stable codes (`incan check --format json`, `incan explain INCAN-T0001`). Generated Rust that fails to compile is treated as a compiler bug | typed AST, IR (records `VarAccess` move/read), ownership planner, emitters, `prettyplease`, rustc | Very active (pushed 2026-11-17), beta v0.5, ~74 stars |
| **Galvan** ([repo](https://github.com/antoniusnaumann/galvan-lang)) | Swift/Kotlin-ish, semicolon inference, `\(x)` interpolation | `async`/`.await` (Tokio), `?` spelled as postfix `!` | Pass-by-value by default with "copy-on-write-style defaults". `mut` parameters become `&mut`. `ref` values have "heap-backed reference semantics". Immutable references are not exposed | Crate paths `mycrate::item`, `use mycrate`. Dependency interop "lifted" from rustdoc JSON (`galvan-rustdoc/LIFTINGS.md`) | Not documented | Transpiles to Rust | Hobby/WIP (pushed 2026-10-20, 5 stars) |
| **Ruchy** ([repo](https://github.com/paiml/ruchy), [spec](https://github.com/paiml/ruchy/blob/main/docs/SPECIFICATION.md)) | Python/Swift-like, `fun`, f-strings | `match`, labeled loops, Rust closures `\|x\|`. The spec's contract says "ALWAYS preserve Rust's ownership and borrowing rules" | Swift model: `struct` is a value type (derives `Clone`), `class` becomes `Rc<RefCell<_>>` or `Arc<Mutex<_>>`. Has an interpreter mode as well | Uses Rust std and crates directly (spec 1.9) | `ruchy check` / `ruchy lint`. Mapping of rustc errors is not documented | Parser, desugar, `RustCodeGen`, rustc. Also a REPL and wasm | Active (pushed 2026-11-16), v5.0.0-beta, ~62 stars |
| **Fable (Rust target)** ([repo](https://github.com/fable-compiler/Fable), [`fable-library-rust`](https://github.com/fable-compiler/Fable/tree/main/src/fable-library-rust/src)) | F# | Static typing | F# reference semantics run on a runtime library (`LrcPtr.rs`, a ref-counted pointer, plus `Mutable.rs`, `Native.rs`). Not idiomatic Rust | `Interop.rs` / emit attributes | F# compiler diagnostics. The emitted Rust is expected to compile | F# AST, Fable AST, Rust AST, rustc | Fable itself is very active. The Rust target is one of six backends |
| **py2many / pyrs** ([py2many](https://github.com/py2many/py2many), [pyrs](https://github.com/konchunas/pyrs)) | Python | n/a | Converts Python one way, as a migration aid. Not a language | none | none | Python AST to Rust text | py2many maintained. pyrs archived |
| **Copper, Crisp, Deor, Crusty, Gust** ([copper](https://github.com/liy77/copper-lang), [crisp](https://github.com/jose-compu/crisp), [deor](https://github.com/nathanphoffman/DeorLang)) | JS-ish, Python-ish, C-like, state machines | varies | Mostly implicit or hidden ownership | Copper supports sibling `.rs` files. Deor has a `rust {}` escape block | n/a | Transpile to Rust | 0 to 5 stars, early stage |

No language with a Ruby- or Elixir-style surface that compiles to Rust turned up. The closest Ruby result, [aloxide](https://github.com/oceanpkg/aloxide), works the other way round: it builds Ruby from a crate's `build.rs`.

### B. Rust-flavoured languages that do not compile to Rust

| Language | What it is | Relevance |
|---|---|---|
| **Borgo** ([repo](https://github.com/borgo-lang/borgo)) | "A statically typed language that compiles to Go". Rust-like syntax, ADTs, `match`, `Result` with `?`, and full interop with Go packages | A *syntax* layered over a different host's semantics. Its interop works because Borgo maps onto Go's semantics instead of fighting them, which is the same bet vai makes with Rust. Dormant (last push 2024-10-27, 4.6k stars) |
| **Lisette** ([repo](https://github.com/ivov/lisette)) | "A little language inspired by Rust that compiles to Go". Promises readable Go output and Go-ecosystem interop | Same pattern as Borgo, still alive (pushed 2026-11-18, 1.6k stars). Its "readable Go" goal matches vai's readable Generated Rust |
| **Dada** ([repo](https://github.com/dada-lang/dada)) | Niko Matsakis's "thought experiment": a Rust that feels like Java/JS, with a minimal runtime | Redesigns ownership itself instead of wrapping Rust. Its compiler uses Salsa for incremental work, has an LSP crate, and its codegen depends on `wasm-encoder`, so it targets WebAssembly, not Rust. README: "Dada doesn't really exist". Last push 2026-10-10. Shows how far you have to go to make ownership feel lighter, which vai has ruled out |
| **Rune** ([repo](https://github.com/rune-rs/rune)) | "An embeddable dynamic programming language for Rust". VM, hot reloading, macros | Its interop is a host embedding API (modules registered from Rust), not crate-level. Dynamic semantics. Not a model for vai |
| **Mun** ([repo](https://github.com/mun-lang/mun)) | AOT via LLVM, static types, first-class hot reloading, embedded through a Rust runtime | Has its own memory model (structs can be "garbage collected", per its README). Interop goes through runtime marshalling, not crates |
| **Rhai, Gluon** ([rhai](https://github.com/rhaiscript/rhai), [gluon](https://github.com/gluon-lang/gluon)) | Embedded scripting languages for Rust | Same category as Rune |
| **Verus** ([repo](https://github.com/verus-lang/verus)) | Verified Rust. Its extra syntax lives inside a `verus! { }` macro, and a custom rustc driver checks it | The other way to change Rust's surface: stay inside rustc. Spans come for free because the code passes through rustc's own token stream. Shows that a macro-embedded surface keeps diagnostics native |

## Diagnostics: the upstream state

- rustc has no way to say "this generated line came from `foo.vai:12:3`".
- Proposals so far:
  - [pre-RFC: supporting `#line` directives (2017)](https://internals.rust-lang.org/t/pre-rfc-supporting-line-directives-either-directly-or-with-an-extension/4765)
  - [pre-RFC: enable setting of source file, line & column (2023)](https://internals.rust-lang.org/t/pre-rfc-enable-setting-of-source-file-line-column/19156)
  - [pre-RFC: `#[line]` attribute (June 2024)](https://internals.rust-lang.org/t/pre-rfc-line-attribute/21069)
- The 2024 thread closed automatically in September 2024. josh said the team "would generally like this to happen", but these questions were left open:
  - where the mapping lives (inline vs. an external source map)
  - how it interacts with macros and `line!()`
  - whether it can work below item level, which yacc-style generators need
- In practice, the Linux kernel uses a rustc wrapper script that rewrites diagnostics for generated doctests, appending "generated from original_file:original_line" ([kunit-dev patch](https://groups.google.com/g/kunit-dev/c/i9XjNVAJQ1A)). This is a post-processing approach.

That leaves vai three approaches. Choosing between them belongs to the pipeline ticket:

1. **Front-end checking, rustc hidden** (Incan, Fable). Not enough for vai, because borrow-check errors exist only in rustc.
2. **Emit Rust, run `rustc --error-format=json`, and remap spans** through a vai-owned source map (the kernel's approach). Works on stable today. vai must own the remapping and the rendering.
3. **Lower through a proc macro / token stream with spans that point into `.vai` files** (closest to Verus). rustc then reports real locations natively. Limits: proc-macro spans can't easily name a non-`.rs` file on stable, and the Generated Rust stops being an inspectable file.

## Lessons for vai

1. **vai's niche is real but unoccupied.** Every competitor gave up Rust's 1:1 semantics to get ergonomics: hidden clones, `Rc`/`Arc` classes, copy-on-write. vai's standing decision ("no GC, no hidden borrow checker") is its distinguishing feature. Expect users to ask "why not just auto-clone like Incan?", and have the answer written into the spec.
2. **Pain point #1 (lifetimes) has to be solved with syntax, not by planning ownership.** Incan's duckborrowing is roughly 190 lines of contributor rules for a planner that is "not a second Rust borrow checker", yet it still clones at match scrutinees and adds `Clone` bounds the user never wrote. vai can borrow Galvan's call-site markers (`f(mut x)`, `x.mut`), which make `&mut` visible but quiet, without Galvan's heap-backed `ref` semantics.
3. **Diagnostic mapping is vai's hardest infrastructure problem.** None of the surveyed projects has solved it for borrow-check errors, and upstream won't solve it soon. Settle it before the syntax, because it constrains how Generated Rust is laid out: one item per region, stable spans, a source map format.
4. **Crate interop prior art:**
   - Incan's `import rust::crate` prefix and Galvan's rustdoc-JSON "liftings" both show that a typed front end needs crate metadata. vai may not need this if it leaves type checking to rustc.
   - Incan's Cargo-free toolchain creates friction for the "consume vai from Rust" direction. vai should stay plain Cargo (`build.rs` or a cargo subcommand) to keep mixed `.vai` and `.rs` crates trivial.
5. **Treat readable Generated Rust as a contract.** Incan's "Generated Rust Quality" doc and Lisette's "readable Go" line both make output quality a product surface:
   - stable identifiers (Incan reports declarations under reversible `incan-v1` identifiers instead of their source names)
   - `prettyplease` formatting
   - an inspect command
   
   vai's golden corpus plays the same role and should be written as one.
6. **Identifier mangling needs a rule.** Incan has to map Rust names that are reserved words in Incan (`import type as proto_type`), and Galvan "label-mangles" overloads. vai's Ruby-style `?`/`!` method names will need a reversible mapping in both directions so Rust callers see sensible names. This feeds the naming-conventions fog item.
7. **Borgo and Lisette show the bet can pay off.** A familiar syntax over another language's unchanged semantics drew thousands of stars, and interop came "for free" because they didn't fight the host. That supports vai's strategy. Borgo's dormancy also shows the risk: a solo project with no story for growing beyond its author.

## Open questions surfaced

- Pipeline ticket: of the three diagnostic approaches, which one? Does approach 3 (proc-macro spans) even work for non-`.rs` files on stable?
- Can rustc's JSON diagnostics be remapped well enough for multi-span borrow errors (labels, notes, suggestions in Rust syntax)? How do vai's diagnostics phrase rustc's `help:` suggestions, which are written in Rust spelling?
- Does vai need any type information of its own (e.g. to choose `&` vs `&mut` for Sugar), or is it purely syntactic? If it is purely syntactic, it needs no crate metadata ingestion.
- Rust-to-vai direction: what do Rust callers see for vai `?`/`!` names and atoms?
