# Compiler pipeline options for vai → Rust

Research for [#3](https://github.com/marcelocg/vai/issues/3). This compares the options and makes no choice; the decision belongs to [#14](https://github.com/marcelocg/vai/issues/14).

Terms follow [GLOSSARY.md](../GLOSSARY.md): *vai source*, *Generated Rust*, *Lowering*, *Crate interop*.

## TL;DR

| | (a) Source-to-source + build.rs / `cargo vai` | (b) Proc macro `vai! { }` | (c) `rustc_driver` frontend |
|---|---|---|---|
| Toolchain | Stable | Stable | Nightly only, pinned to one nightly (`rustc_private`, `rustc-dev` + `llvm-tools` components) |
| API stability | Cargo build-script contract + plain Rust text. Both stable | `proc_macro` stable API. Span line/column/file stable since 1.88 | Internal compiler APIs. "Always going to be unstable" |
| Input fidelity | Own lexer, so any syntax works | vai source must lex as **Rust tokens** (balanced delimiters, Rust literals; `#` comments and `'str'` break it) | Own lexer, so any syntax works |
| Mixed .vai/.rs crate | Yes: `.rs` in `src/`, `.vai` lowered into `OUT_DIR`, pulled in with `include!` | Yes, trivially: vai lives inside `.rs` files | Yes, but every crate that builds vai needs the custom driver |
| Downstream consumers (Crate interop) | Stable build-dependency on the vai compiler, or ship the Generated Rust | Stable proc-macro dependency | Dependents can't build the crate without the driver unless you publish the lowered Rust |
| Incremental | build.rs reruns when `rerun-if-changed` paths change (mtime). rustc incremental works on the generated text | Re-expands on each crate compile. Can't cheaply track external `.vai` files on stable | Native rustc incremental, but pinned to one nightly |
| Diagnostics | rustc reports positions in the Generated Rust. Mapping back needs a sidecar map and a wrapper that rewrites `--message-format=json` (Rust has no `#line`) | Output tokens can carry spans from the input, so errors land on the vai text inside the `.rs` file. Works only for inline vai, not separate `.vai` files | The driver can register `.vai` files in rustc's SourceMap and build AST with real spans, so errors point at `.vai` natively |
| Effort | Lowest. Parser + Lowering + printer, plus a small build helper | Low-medium. Same Lowering, emitted as `TokenStream`. Lexing is constrained | Highest. Track nightly churn, distribute a toolchain-matched binary |

## (a) Source-to-source transpiler emitting `.rs`

**Mechanism.** A `vai` compiler (a library plus a CLI) parses `.vai` and prints Generated Rust. You can drive it in two ways.

- **build.rs.** The crate has `build-dependencies = { vai-build = "…" }`. `build.rs` lowers `src/**/*.vai` into `OUT_DIR` and `.rs` code pulls the result in with `include!(concat!(env!("OUT_DIR"), "/foo.rs"))`. This is the same pattern as prost and lalrpop (`lalrpop_mod!` wraps that `include!`). Crusty, a C-like language, ships the same build.rs integration ([github.com/major0/crusty](https://github.com/major0/crusty)).
- **Cargo subcommand.** Cargo runs any `cargo-<name>` binary on `PATH` as `cargo <name>`, and recommends it drive Cargo through the CLI using the `CARGO` env var ([Cargo book: External tools](https://doc.rust-lang.org/cargo/reference/external-tools.html)). A `cargo vai build` could lower the sources, invoke `cargo build --message-format=json` and rewrite the diagnostics.

You can combine the two: build.rs does the generation, so plain `cargo build` works, and `cargo vai` is an optional layer that maps diagnostics.

**Facts from the sources.**

- Build scripts "may save any output files … in the directory specified in the `OUT_DIR` environment variable. Scripts should not modify any files outside of that directory." Cargo doesn't clean `OUT_DIR` between builds. ([Cargo book: Build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html))
- `cargo publish` enforces the `OUT_DIR` rule. Since [cargo#5584](https://github.com/rust-lang/cargo/pull/5584), verification fails with "Source directory was modified by build.rs during cargo publish". So Generated Rust can't be written next to the sources during the build. Checking it into `src/` requires a separate, explicit `vai lower` step run outside the build.
- `rerun-if-changed` compares mtimes and scans whole directories. Without any `rerun-if` line, Cargo reruns the script whenever any package file changes. ([Build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html))
- `include!` parses a file "as an expression or an item according to the context", unhygienically. Paths resolve relative to the current file. ([std::include](https://doc.rust-lang.org/std/macro.include.html)) The practical consequence is that the generator should emit inline `mod x { … }` and avoid out-of-line `mod x;` declarations inside included files.
- Rust has no `#line` / source-location directive. Pre-RFCs keep coming up ([2017](https://internals.rust-lang.org/t/pre-rfc-supporting-line-directives-either-directly-or-with-an-extension/4765), [2023](https://internals.rust-lang.org/t/pre-rfc-enable-setting-of-source-file-line-column/19156), [`#[line]` 2024](https://internals.rust-lang.org/t/pre-rfc-line-attribute/21069)), but none has landed. Mapping diagnostics back to `.vai` is therefore the tool's job. It can do this by post-processing rustc JSON (`compiler-message` objects carry rustc spans; [External tools](https://doc.rust-lang.org/cargo/reference/external-tools.html)) against a source map the lowering step emits.

**Mixed crates.** These are natural here. `.rs` modules sit in `src/` as usual. Each `.vai` module becomes either a generated file that a small stub includes, or a single generated `vai_mods.rs` that is included at the crate root. Rust code calls vai code as ordinary Rust items, and vai code calls Rust because the Generated Rust is plain Rust.

**Crate interop downstream.** Dependents compile the published crate on stable. A build-dependency on the vai compiler is enough, and that build-dep compiles on stable. The alternative is to publish the lowered Rust.

**Tooling.** rust-analyzer runs build scripts and understands `include!` of `OUT_DIR` files. IDE features land in the Generated Rust, not in `.vai` (no vai LSP is in scope).

**Effort.** Lowest, and it fits the project's spec deliverable best: the golden corpus of `.vai` → Generated Rust pairs is exactly this compiler's test suite.

## (b) Proc-macro embedding (`vai! { … }`)

**Mechanism.** A `#[proc_macro]` crate receives the invocation's inner `TokenStream` and returns Rust tokens that replace the invocation ([Reference: Procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html)).

**Facts from the sources.**

- The input is already-lexed Rust token trees: groups, punctuation, literals and identifiers. Doc comments arrive as `#[doc]` attributes ([Reference](https://doc.rust-lang.org/reference/procedural-macros.html)). It follows that vai syntax inside `vai!` must survive rustc's lexer. Delimiters must balance. `#` starts no comment: a vai `# don't` comment fails to lex because of the apostrophe. Ruby-style `'single quoted'` strings don't lex either. Whitespace and newlines aren't tokens, so a newline/indent-sensitive grammar has to reconstruct layout from spans.
- Since **1.88**, `Span::line`, `column`, `start`, `end`, `file` and `local_file` are stable, and so is `source_text` (since 1.66). That makes layout recoverable on stable. `Span::join`, `byte_range` and the `Diagnostic` API (`error`/`warning`/`note`) remain nightly-only (`proc_macro_span`, `proc_macro_diagnostic`). ([proc_macro::Span](https://doc.rust-lang.org/proc_macro/struct.Span.html)) On stable, errors are reported via `compile_error!` or a panic.
- `Span` has no public constructor for arbitrary file positions. Spans exist only as call/mixed/def site, or copied from input tokens. So a macro that reads an external `foo.vai` (`vai::include!("foo.vai")`) can't point diagnostics into that file. Rebuild tracking for such a file (`proc_macro::tracked::path`) is nightly-only ([tracking issue #99515](https://github.com/rust-lang/rust/issues/99515); [proc_macro::tracked](https://doc.rust-lang.org/proc_macro/tracked/index.html)). The common stable workaround is to emit `include_str!` of the file so rustc records the dependency.
- Proc macros are unhygienic ([Reference](https://doc.rust-lang.org/reference/procedural-macros.html)). That suits vai, because Generated Rust *should* behave as if it were written inline.
- Precedent: Lia (`lia!`) compiled a language to Rust via a proc macro and compiler plugins on nightly, and is abandoned ([github.com/willcrichton/lia](https://github.com/willcrichton/lia)). DSL macros such as `html!`/`rsx!` show the approach working for embedded sub-languages, not for whole-program syntax.

**Mixed crates.** Yes by construction, but the *unit* is a macro block inside a `.rs` file, not a `.vai` file. Full `.vai` files would need the `include!`-style macro above and would lose spans and stable rebuild tracking.

**Diagnostics.** This is (b)'s main advantage. If the Lowering copies input spans onto output tokens, rustc type and borrow errors point at the vai text inside the `.rs` file, with no mapping layer.

**Effort.** Low to medium. Lowering is shared with (a) if the compiler core emits a Rust AST/tokens (for example via `quote`/`proc-macro2`). The constrained lexing pushes on language design decisions such as comments, string quotes and layout.

## (c) `rustc_driver`-based frontend

**Mechanism.** A custom binary links `rustc_driver` and `rustc_interface`, implements `rustc_driver::Callbacks` and runs the compiler. Cargo can route workspace crates through it via `build.rustc-workspace-wrapper` / `RUSTC_WORKSPACE_WRAPPER`, which also gets separately hashed artifacts. This is how clippy-driver works. ([Cargo config](https://doc.rust-lang.org/cargo/reference/config.html))

**Facts from the sources.**

- "By its very nature, the internal compiler APIs are always going to be unstable." ([rustc-dev-guide: rustc_driver](https://rustc-dev-guide.rust-lang.org/rustc-driver/intro.html)) Using them needs `#![feature(rustc_private)]`, so nightly only, plus `rustup component add rustc-dev llvm-tools`, typically pinned in `rust-toolchain.toml` to one dated nightly ([rustc-dev-guide: External rustc drivers](https://rustc-dev-guide.rust-lang.org/rustc-driver/external-rustc-drivers.html), [Unstable book: rustc_private](https://doc.rust-lang.org/nightly/unstable-book/language-features/rustc-private.html)). The guide's own example says "Tested with nightly-2025-03-28".
- Hooks available: `config(&mut Config)`, which includes a custom `file_loader`; `after_crate_root_parsing(&Compiler, &mut ast::Crate)`, which gives a mutable AST, but "submodules are not yet parsed"; `after_expansion`; `after_analysis` (read-only `TyCtxt`). ([Callbacks](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_driver_impl/trait.Callbacks.html), [Interacting with the AST](https://rustc-dev-guide.rust-lang.org/rustc-driver/interacting-with-the-ast.html))
- **"Producing HIR/MIR directly" is not an offered entry point.** HIR comes from rustc's own AST lowering and MIR from HIR. The realistic injection points are (i) `file_loader`, which serves Generated Rust text for `.vai` paths and amounts to option (a) inside rustc, or (ii) building `rustc_ast` nodes with spans into `.vai` files registered in the SourceMap. Option (ii) is the one that gives native `.vai` diagnostics. `rustc_public` (formerly Stable MIR) aims at a stable *analysis* API for tools, not at feeding code in ([project-stable-mir](https://rust-lang.github.io/rustc_public/)).

**Mixed crates.** These are possible: `.rs` parses normally and `.vai` modules are injected. But every compile of the crate needs the driver. That includes downstream dependents building it from crates.io, which conflicts with "consume vai code from Rust with no wrapper layer" unless the published crate ships lowered Rust, at which point it is (a).

**Effort.** Highest. Ongoing work to follow nightly API churn, toolchain-matched binaries to distribute (as rustup does for clippy/miri), and no stable story for users.

## Cross-cutting observations

1. **The Lowering core is shared.** A `vai → Rust AST/tokens` library serves (a) as a printer, (b) as `TokenStream` output and (c)(i) as text. The architectural choice is mostly about the *driver* and the *diagnostics*, not the compiler core.
2. **Diagnostics decide between the options.** (a) needs an explicit source map and a JSON-rewriting wrapper. (b) gets spans for free, but only for inline vai. (c) gets them natively, at the cost of nightly. Rust offers no `#line` escape hatch.
3. **Hybrids are precedented.** cxx pairs a build-time generator (`cxx-build` in build.rs) with a proc macro (`#[cxx::bridge]`). vai could use build.rs for `.vai` files and a `vai!` macro for small inline blocks or the raw-Rust escape hatch in the other direction.
4. **Only (a) and (b) work on stable.** That matters because the map requires Crate interop in both directions.

## Sources

- Cargo book: [Build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html), [External tools](https://doc.rust-lang.org/cargo/reference/external-tools.html), [Configuration](https://doc.rust-lang.org/cargo/reference/config.html)
- [cargo#5584](https://github.com/rust-lang/cargo/pull/5584): publish verification of build.rs source modification
- Rust reference: [Procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html)
- std: [`include!`](https://doc.rust-lang.org/std/macro.include.html), [`proc_macro::Span`](https://doc.rust-lang.org/proc_macro/struct.Span.html), [`proc_macro::tracked`](https://doc.rust-lang.org/proc_macro/tracked/index.html)
- rustc-dev-guide: [rustc_driver intro](https://rustc-dev-guide.rust-lang.org/rustc-driver/intro.html), [Interacting with the AST](https://rustc-dev-guide.rust-lang.org/rustc-driver/interacting-with-the-ast.html), [External rustc drivers](https://rustc-dev-guide.rust-lang.org/rustc-driver/external-rustc-drivers.html)
- [`rustc_driver_impl::Callbacks`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_driver_impl/trait.Callbacks.html); [Unstable book: rustc_private](https://doc.rust-lang.org/nightly/unstable-book/language-features/rustc-private.html); [rustc_public](https://rust-lang.github.io/rustc_public/)
- Internals pre-RFCs on line directives: [2017](https://internals.rust-lang.org/t/pre-rfc-supporting-line-directives-either-directly-or-with-an-extension/4765), [2023](https://internals.rust-lang.org/t/pre-rfc-enable-setting-of-source-file-line-column/19156), [2024](https://internals.rust-lang.org/t/pre-rfc-line-attribute/21069)
- Existing projects: [Crusty](https://github.com/major0/crusty) (build.rs transpiler), [Lia](https://github.com/willcrichton/lia) (proc-macro language, abandoned)
