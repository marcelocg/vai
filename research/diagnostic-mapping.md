# Mapping rustc diagnostics back to vai source

Research for [#4](https://github.com/marcelocg/vai/issues/4). Feeds the pipeline decision in [#15](https://github.com/marcelocg/vai/issues/15). Facts and tradeoffs only; no strategy is chosen here.

## Question

How can rustc errors, especially borrow-checker and type errors, be reported against vai source spans instead of Generated Rust? What fidelity can we realistically get?

## Short answer

- **Rust has no `#line` directive and no source-map input.** It has been proposed three times on internals (2017, 2023, 2024). The lang team said it "would generally like to see it happen", but there is no RFC and no implementation. External source maps are the option the discussion leans towards long term.
- **Two mechanisms work today:**
  1. **Out-of-process remapping.** Run `rustc --error-format=json` or `cargo --message-format=json`, map every span through a vai-owned span table, and re-render the diagnostic ourselves. This works for standalone `.vai` files on stable Rust.
  2. **In-process spans through a proc macro.** Generated tokens can carry the spans of the vai tokens they come from, so rustc reports errors on the vai text itself, but only when the vai text is the macro's input, which lives inside a `.rs` file. It does not work for standalone `.vai` files: a stable proc macro cannot make a span that points into any other file.
- **What fidelity to expect:** span locations can be mapped precisely, at sub-expression level, if Lowering records origins. Message *text* cannot. rustc's messages, labels and `suggested_replacement`s use Rust names, types and syntax (`&mut`, `::`, turbofish, renamed identifiers). We can rewrite them with heuristics but cannot translate them faithfully in general. Runtime locations (panic `Location`, DWARF line tables, backtraces) stay on Generated Rust under either mechanism.

## 1. What rustc gives us: the JSON diagnostic format

Source: [rustc book, "JSON Output"](https://doc.rust-lang.org/rustc/json.html).

- `--error-format=json` writes one diagnostic per line to stderr, tagged `"$message_type": "diagnostic"`.
- A diagnostic has `message`, `code` (`{code, explanation}`, may be null), `level`, `spans[]`, `children[]` and `rendered`. Children have the same shape but are "not nested", so the tree is one level deep.
- Each span has:
  - `file_name`, which "may not exist".
  - `byte_start`/`byte_end`: 0-based, with `byte_end` exclusive.
  - `line_start`/`line_end`: 1-based.
  - `column_start`/`column_end`: 1-based, counted in Unicode Scalar Values, not bytes.
  - `is_primary`, `label`, `text[]`, `suggested_replacement` and `suggestion_applicability` (`MachineApplicable` / `MaybeIncorrect` / `HasPlaceholders` / `Unspecified`).
  - `expansion`, which is non-null inside a macro expansion and holds `{span, macro_decl_name, def_site_span}`.
- Stability: the format is documented, but consumers must stay forward-compatible: "New fields may be added. Enumerated fields … may add new values." It is not versioned.
- Under Cargo, `--message-format=json` wraps each diagnostic as `{"reason":"compiler-message", package_id, manifest_path, target, message}` on stdout ([Cargo book, External tools](https://doc.rust-lang.org/cargo/reference/external-tools.html)). Proc-macro or build-script output can be mixed in, so Cargo itself advises treating only lines that start with `{` as JSON.

What this means for vai:
- Every span carries both byte offsets and line/column. A vai span table keyed by byte ranges in the generated file can therefore map every primary and secondary span, including the many labelled spans of borrow-check errors such as "first mutable borrow occurs here" and "second borrow occurs here".
- `rendered` is pre-formatted text pointing at the `.rs` file, so it is useless once spans are remapped. vai must re-render from the structured fields, for example with a snippet-rendering crate.
- `expansion` data shows when an error sits inside a Rust macro that vai called (in v0 scope: "calling Rust macros"). Such errors can be mapped to the vai macro call site.

## 2. In-compiler alternatives: `#line`, `#[line]` and source-map input

None exist in stable or nightly rustc.

- **2017** [pre-RFC: Supporting `#line` directives](https://internals.rust-lang.org/t/pre-rfc-supporting-line-directives-either-directly-or-with-an-extension/4765): motivated by an m4-generated state machine.
- **2023** [pre-RFC: Enable setting of source file, line & column](https://internals.rust-lang.org/t/pre-rfc-enable-setting-of-source-file-line-column/19156): motivated by transpiled C/Java and HTML templates compiled to Rust. Key points:
  - CAD97: "There's currently not any way to create a custom span", and "External sourcemaps is probably the only solution which doesn't lead to endless problems."
  - Afetisov: a generated `a + b` may have no clean single origin span.
  - No RFC followed.
- **2024** [Pre-RFC: `#[line]` attribute](https://internals.rust-lang.org/t/pre-rfc-line-attribute/21069): `#[line = N]` on an item, module or expression. josh said the team "would generally like to see it happen" and was "tempted by the idea of feeding a source map into the compiler". kornel prefers source maps long term but sees `#[line]` as a simple first step. The thread auto-closed in September 2024 with no RFC PR.

What this means for vai: building on a future `#[line]` or source-map flag is speculative. Even if `#[line]` lands, it is line-granular and would probably also change `line!()` at runtime (bjorn3's point in the 2024 thread). A vai design should work without it and treat it as a possible later improvement, mostly for panic and debuginfo locations.

## 3. Proc-macro span tricks

Source: [`proc_macro::Span` docs](https://doc.rust-lang.org/proc_macro/struct.Span.html).

Stable:
- `call_site` (1.29) and `mixed_site` (1.45).
- `resolved_at` and `located_at` (1.45): take hygiene from one span and location from another.
- `source_text` (1.66): "best effort", for diagnostics only.
- `start`, `end`, `line`, `column`, `file`, `local_file` (1.88): these *read* locations.

Nightly only:
- `def_site` (#54724).
- `join`, `byte_range`, `parent`, `source` (#54725).
- `Diagnostic` with `error`/`warning`/`note`/`help` (#54140).

There is no constructor for a span at an arbitrary file, line or column. Spans can only come from tokens the macro received, or from the call site.

Consequences:
- **Inline vai inside a `.rs` file (`vai! { … }`).** Each generated token can be given the span of the vai token it came from (`located_at`, or `quote_spanned!`). rustc then reports borrow and type errors directly on vai text, with rustc's own rendering, in IDEs, with no post-processing. The cost is that vai text must survive Rust's tokenizer: balanced delimiters, Rust literal and lifetime rules, and no `#`-comments as Ruby has them. That limits the surface syntax, and it is the opposite of the standalone `.vai` files the map assumes.
- **Proc macro that reads a `.vai` file (`vai::include!("x.vai")`).** This cannot point spans into `x.vai`. The best it can do is point at the macro call, which is worse than remapping.
- **Spans of generated code with no vai origin.** Sugar expansions, inserted `&`/`&mut`, and lifetime annotations still need a fallback span, typically the enclosing vai construct. The same problem appears in the remapping approach.

## 4. Source maps (ECMA-426)

Source: [ECMA-426 Source Map Format](https://tc39.es/ecma426/).

- The format was standardized from the de-facto "v3" format; `version` is always 3.
- `mappings` is a base64-VLQ string: lines are separated by `;` and segments by `,`. Each segment is 1, 4 or 5 fields: generated column, source index, original line, original column, and an optional `names` index. Values are relative to the previous segment.
- It also has `sources`, `sourcesContent`, `names`, `ignoreList`, and "index maps" with `sections`.
- Mappings are **points**, not ranges: each generated position maps to an original start position. A range in generated code is mapped by looking up its start and end positions.

Relevance:
- A vai span table could reuse this format, which gives free tooling and a known format if rustc ever accepts source maps (the 2023 and 2024 threads both point that way).
- A custom table keyed by generated byte range and holding a vai byte range plus a "construct kind" fits rustc's byte-offset spans better. It also lets the re-renderer know when a span was synthesized by Sugar.
- The two are not exclusive: we could keep an internal table and export ECMA-426 maps later for debuggers.

## 5. Prior art in other transpiled languages

| Language → target | Mechanism | Notes |
|---|---|---|
| **TypeScript → JS** | `sourceMap`, `inlineSourceMap`, `declarationMap` ([tsconfig reference](https://www.typescriptlang.org/tsconfig/#sourceMap)) | Compile errors never come from the target: tsc type-checks TS itself. Source maps serve runtime debugging and editor go-to-definition only. |
| **Nim → C** | `--lineDir:on` emits C `#line` directives ([nimc docs](https://nim-lang.org/docs/nimc.html)); `--stackTrace`/`--lineTrace` add runtime Nim positions | Nim type-checks before C codegen, so C compiler errors are rare and counted as compiler bugs. `#line` exists mainly for gdb/debuginfo. |
| **Haxe → JS/PHP/C++/…** | `-D source-map` (js, php), `source-map-content`; `annotate-source` adds comments to C++ output ([Haxe compiler flags](https://haxe.org/manual/compiler-usage-flags.html)) | Haxe type-checks itself. Maps are for runtime debugging. |
| **C preprocessors / yacc / Vala-style generators → C** | C `#line` directives ([GCC Line Control](https://gcc.gnu.org/onlinedocs/gcc-10.1.0/cpp/Line-Control.html)) | The C compiler reports errors and debuginfo at the original file and line. This is the feature Rust lacks. |
| **Borgo → Go** | Not documented in the [README](https://github.com/borgo-lang/borgo) | Borgo is written in Rust and is statically typed with its own inference. Nothing found on mapping Go errors back. |
| **venus-core (Rust → Rust)** | Line map plus parsing of rustc JSON ([source on docs.rs](https://docs.rs/venus-core/0.1.0/src/venus_core/compile/errors.rs.html)) | A small existing example of the remapping approach for rustc. It is line-granular only; column mapping is "reserved for future". |

The pattern across these languages: transpilers that **own the type system** check before codegen, so target-compiler errors are rare and source maps exist for *runtime*. vai has the opposite constraint. Its semantics are Rust's 1:1, so vai will **not** run its own borrow checker. rustc's borrow and type errors are the *normal* user-facing errors, not compiler bugs. No prior art above does high-fidelity remapping of a target compiler's semantic errors as its main UX. vai would need to build this.

## 6. Realistic fidelity

| Aspect | Achievable? | Notes |
|---|---|---|
| Primary span location | Yes, precisely | Requires Lowering to keep an origin span on every generated node, and the emitter to record byte ranges. |
| Secondary and labelled spans (borrow-check "first borrow here", …) | Yes | Same table. Each JSON span is mapped independently. |
| Spans on Sugar-synthesized code (inserted `&mut`, lifetimes, `?`, generated `match`) | Partial | Map to the enclosing vai construct. The label may then point at something the user never wrote. |
| Message text and labels | Partial | They are Rust-phrased: types (`&'a mut Vec<T>`), paths with `::`, and identifiers renamed by naming rules (e.g. `empty?` → `is_empty`). Rewriting needs a reverse naming map and type pretty-printing in vai syntax. Heuristic; brittle across rustc versions. |
| `suggested_replacement` / fix-its | Mostly no | They are Rust code. They can be shown as "Rust suggests: …", or translated case by case for a few common codes (e.g. "consider borrowing"). |
| Error codes and `--explain` | Yes, as-is | `code.code`, such as E0502, passes straight through. The explanations are written in Rust. |
| Errors in raw-Rust escape hatch and mixed `.rs` files | Yes | Spans in the escape hatch map to the embedded text by an offset. Spans in plain `.rs` files pass through unchanged. |
| Errors inside called Rust macros | Yes | `expansion.span` gives the call site. |
| Panic locations, backtraces, DWARF/debugger | No (today) | Point to Generated Rust. Readable Generated Rust (a standing decision) softens this. Making the layout line-correlated with vai helps: one vai statement per Rust line, in the same order. `--remap-path-prefix` only rewrites path prefixes, not lines. |
| IDE / rust-analyzer live diagnostics | No, without extra work | rust-analyzer would see `.rs` files. LSP is out of scope for v0. |

## 7. Options for #15 (tradeoffs, not a decision)

1. **Generate `.rs`, run rustc, remap JSON (out-of-process).**
   - For: works on stable, supports standalone `.vai` files, and keeps every byte of message structure.
   - Against: vai owns a renderer and a span table, and depends on the JSON format staying stable. Text rewriting is best-effort.
2. **vai as a proc macro over inline token streams.**
   - For: native rustc rendering on vai text.
   - Against: vai syntax must be a valid Rust token stream, which conflicts with the Ruby/Elixir surface, and it does not work for standalone files.
3. **Generate `.rs` with a line-correlated layout and no remapping**, letting users read readable Generated Rust.
   - For: cheapest.
   - Against: errors point at a file the user didn't write. Works only as a fallback or bootstrap stage.
4. **Hybrid.** Option 1 for compile errors, plus a layout convention (stable line correlation and a `// vai: file:line` comment per item) to help with runtime and debugger locations. Adopt `#[line]` or a source-map flag if rustc ever ships one.

Cross-cutting requirement whatever the choice: **Lowering must keep origin spans on every node**, and the emitter must produce a span table. That needs to be decided at the AST/IR design level, not bolted on.

## Open questions surfaced

- Granularity of the span table: per token, per expression or per statement. Representation: custom, or ECMA-426.
- Which rustc message text (types, paths, renamed identifiers) vai rewrites into vai syntax, and how a reverse naming map interacts with the naming-conventions work.
- How a Sugar construct declares its "blame span" for errors in code it synthesizes.
- Is a line-correlated Generated Rust layout worth requiring, given that it constrains rustfmt-style readability?
- Should vai track and contribute to the `#[line]` / source-map work in rustc?
