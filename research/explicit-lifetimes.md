# Where Rust forces explicit lifetimes

Research for [#5](https://github.com/marcelocg/vai/issues/5), part of the [vai spec v0 map](https://github.com/marcelocg/vai/issues/1).

**Question:** list exactly where Rust's elision rules still require lifetimes in the source, with examples, and say what Rust 2024 and recent RFCs changed. The answer limits what vai can infer and what it has to let the user write.

**Rust version baseline:** stable Rust as of late 2026, edition 2024.

## Sources

- **[REF-ELISION]** Rust Reference, *Lifetime elision*: https://doc.rust-lang.org/reference/lifetime-elision.html
- **[REF-BOUNDS]** Rust Reference, *Trait and lifetime bounds* (HRTBs, implied bounds, `use<..>`): https://doc.rust-lang.org/reference/trait-bounds.html
- **[ED-2024]** Edition Guide, *RPIT lifetime capture rules*: https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html
- **[RFC-141]** RFC 141, lifetime elision: https://rust-lang.github.io/rfcs/0141-lifetime-elision.html
- **[RFC-599/1156]** RFC 599 / RFC 1156, default object lifetime bounds (the Reference cites both)
- **[RFC-2115]** RFC 2115, argument lifetimes (`'_`, impl-header elision; the in-band lifetimes part was not accepted): https://rust-lang.github.io/rfcs/2115-argument-lifetimes.html
- **[RFC-2093]** RFC 2093, inferred outlives requirements on structs (Rust 2018); this is the "inferred for type definitions" passage in [REF-BOUNDS]
- **[RFC-3498]** RFC 3498, lifetime capture rules 2024; **[RFC-3617]** RFC 3617, precise capturing (`use<..>`). Both are summarized in [ED-2024].
- **[LINT]** rustc lint listing, `mismatched_lifetime_syntaxes`: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html
- **[R1.87]** Rust 1.87 release notes (`use<..>` in traits): https://blog.rust-lang.org/2025/05/15/Rust-1.87.0/
- **[R1.89]** Rust 1.89 release notes (`mismatched_lifetime_syntaxes` warns by default): https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/
- **[GAT]** GAT stabilization post (required `where Self: 'a`): https://blog.rust-lang.org/2022/10/28/gats-stabilization/

## 1. The elision rules (what Rust already infers)

**Function signatures [REF-ELISION]**, covering fn items, fn pointers and `Fn*` sugar:

1. Each elided input lifetime becomes its own distinct lifetime parameter.
2. If the inputs use exactly one lifetime (elided or named), every elided output lifetime gets it.
3. In a method whose receiver is `&Self` or `&mut Self`, every elided output lifetime gets the receiver's lifetime.
4. Any other elided output lifetime is an error. There is no inference from the function body; signatures are checked on their own.

```rust
fn substr(s: &str, until: usize) -> &str;                 // = fn substr<'a>(s: &'a str, ..) -> &'a str
fn args<T>(&mut self, args: &[T]) -> &mut Command;        // output tied to self (rule 3)
fn new(buf: &mut [u8]) -> Thing<'_>;                      // '_ in a path: preferred spelling
type F = fn(&str) -> &str;                                // = for<'a> fn(&'a str) -> &'a str
```

**Other places Rust infers or defaults:**

- **Default object lifetime bound** [REF-ELISION, RFC-599/1156]: `Box<dyn Tr>` means `Box<dyn Tr + 'static>`, and `&'a dyn Tr` means `&'a (dyn Tr + 'a)`.
- **`const`/`static`** [REF-ELISION]: a reference type defaults to `'static`, e.g. `const S: &str` means `&'static str`.
- **Impl headers** [RFC-2115]: `impl Trait for Foo<'_>` and `impl Foo<'_>` are stable. `'_` declares a fresh anonymous lifetime.
- **Outlives bounds on type definitions** [RFC-2093, REF-BOUNDS]: `struct S<'a, T> { r: &'a T }` no longer needs `T: 'a`.
- **Implied bounds** [REF-BOUNDS]: a function may assume the well-formedness bounds of its argument and return types, e.g. `&'a T` gives `T: 'a`. Only lifetime bounds are implied, not trait bounds.
- **HRTBs in `Fn` sugar and fn pointers** [REF-ELISION]: `F: Fn(&i32) -> &i32` already means `for<'a> Fn(&'a i32) -> &'a i32`.
- **Function bodies**: local `let` bindings never take lifetime annotations. Region inference (NLL) handles them, so the problem is confined to signatures and type definitions.

## 2. Where lifetimes stay mandatory (the enumeration)

Each case comes with the smallest Rust code that needs the annotation.

### 2.1 Type definitions that hold references

A struct, enum or union that stores a reference, or a type with a lifetime parameter, must declare the parameter. **No elision exists in type definitions.** The Reference also rejects an unused `struct Biz<'a>;` (E0392).

```rust
struct Parser<'src> { input: &'src str, pos: usize }
enum Token<'src> { Ident(&'src str), Num(i64) }
struct Wrap<'a> { p: Parser<'a> }            // propagates: the holder must declare it too
```

The *use sites* of such types can mostly elide: `fn f(p: &Parser) -> Token<'_>`. But see 2.9: the lint now wants `'_` written out.

### 2.2 Several reference inputs and a reference output (no `self`)

Rule 2 needs exactly one input lifetime. With two or more, an output reference is an error [REF-ELISION].

```rust
fn longest(x: &str, y: &str) -> &str;                     // ILLEGAL: E0106
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str;        // must write
fn first<'a>(x: &'a str, _y: &str) -> &'a str;            // or pick one
```

The same applies to `&self` methods whose output borrows from a *non-self* argument. Rule 3 silently ties the output to `self`, so tying it to another argument takes a name:

```rust
impl Cache { fn pick<'k>(&self, key: &'k str) -> &'k str { key } }
```

### 2.3 Reference output with no reference input

There is nothing to elide from [REF-ELISION]. The output is either `'static` or a caller-chosen generic lifetime.

```rust
fn get_str() -> &str;                     // ILLEGAL
fn get_str() -> &'static str { "hi" }     // must write 'static
fn leak<'a>() -> &'a mut Vec<u8>;         // or an unconstrained named lifetime
```

### 2.4 Trait objects whose default bound is wrong or ambiguous

The default bound applies wherever the object appears, and it is often `'static` [REF-ELISION]. Storing a borrowing closure or object requires the bound to be spelled out:

```rust
struct Handler<'a> { cb: Box<dyn Fn(&str) + 'a> }  // without + 'a it means 'static
fn boxed<'a>(x: &'a str) -> Box<dyn Display + 'a> { Box::new(x) }
```

Note the inner-most object rule: `&'a Box<dyn Foo>` is still `&'a Box<dyn Foo + 'static>`.
A containing type with two lifetime bounds is a hard error [REF-ELISION]:

```rust
struct TwoBounds<'a, 'b, T: ?Sized + 'a + 'b> { f1: &'a i32, f2: &'b i32, f3: T }
type T7<'a, 'b> = TwoBounds<'a, 'b, dyn Foo>;      // ERROR: bound cannot be deduced
```

`dyn Tr + '_` exists and follows the fn-elision rules [REF-ELISION]. That makes it the ergonomic spelling inside fn signatures, but it is not allowed in a struct field.

### 2.5 Impl blocks where the lifetime is used more than once or named in the body

`impl Foo<'_>` is enough when the lifetime is never referred to again. It has to be named when:

- the same lifetime appears in both the trait and the self type: `impl<'a> From<&'a str> for Token<'a>`;
- a method signature or an associated type refers to it: `type Item = &'a T;` in `impl<'a, T> Iterator for Iter<'a, T>`;
- an outlives bound mentions it: `impl<'a, T: 'a> ...`.

```rust
impl<'a> Iterator for Words<'a> {
    type Item = &'a str;                         // associated types never elide
    fn next(&mut self) -> Option<&'a str> { .. } // the output borrows from the data, NOT from &mut self
}
```

This last pattern is the clearest cost of elision rule 3. Elision would tie the output to the short `&mut self` borrow, which is wrong for iterators and parsers, so `'a` has to be named.

### 2.6 Associated types and GATs

Associated types in an impl (`type Item = &'a str`) have no elision. GATs need an explicit lifetime parameter, and the compiler *requires* the user to write `where Self: 'a` on the trait [GAT]:

```rust
trait LendingIterator {
    type Item<'a> where Self: 'a;                       // required bound, not inferred
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}
```

The GAT post says the bound is required for now so that it might be implied automatically later.

### 2.7 `'static` and outlives bounds

These are API contracts and never elided:

- `T: 'static` for threads, `Any`, and global registries: `fn spawn<F: FnOnce() + Send + 'static>(f: F)`.
- `T: 'a` and `'a: 'b` when they are not implied by a type in the signature (or the impl header) [REF-BOUNDS]:

```rust
impl<'a, T> Trait<'a, T> for () {}        // ERROR without `T: 'a`; nothing implies it
fn shorten<'a: 'b, 'b>(x: &'a str) -> &'b str { x }
```

### 2.8 Higher-ranked bounds outside `Fn` sugar

`for<'a>` is implicit for `Fn(&T)` and `fn(&T)` [REF-ELISION]. It has to be written when the bound is a non-`Fn` trait, or when a type itself takes the borrow [REF-BOUNDS]:

```rust
fn eq_ref<T>() where for<'a> &'a T: PartialEq<i32> {}
fn parse_all<P>(p: P) where P: for<'a> Parser<'a, Output = &'a str> {}
fn de<T>() where T: for<'de> serde::Deserialize<'de> {}   // serde's DeserializeOwned exists to hide this
```

Closures are a related gap. A closure's own signature does **not** get fn-style output elision. `|x: &i32| -> &i32 { x }` fails because the return lifetime is inferred separately. The fix (`for<'a> |x: &'a i32| -> &'a i32`, closure lifetime binders) is still unstable. Stable code works around it by passing the closure through a function with an `Fn(&T) -> &T` bound. (This is not in the Reference's elision chapter; it comes from the unstable `closure_lifetime_binder` feature.)

### 2.9 Elided-but-hidden lifetimes in paths (lint pressure, Rust 1.89+)

Writing `Parser` for `Parser<'_>` is legal, but since 1.89 `mismatched_lifetime_syntaxes` warns by default [LINT, R1.89]. It fires when an input and an output tied by elision use different syntaxes: named, elided (`&`, `'_`) or hidden (`Iter<u8>`).

```rust
fn items(scores: &[u8]) -> std::slice::Iter<u8>      // warn: hiding a lifetime that's elided elsewhere
fn items(scores: &[u8]) -> std::slice::Iter<'_, u8>  // clean
```

`elided_lifetimes_in_paths` (allow by default, part of the `rust_2018_idioms` group) goes further and wants `'_` on every hidden path lifetime. The result is that idiomatic modern Rust writes `'_` more, not less.

### 2.10 Return-position `impl Trait` (changed in 2024, see section 3)

Before 2024 you had to write `+ 'a` or a `Captures<..>` trick to let a free-function RPIT borrow from an argument. In 2024 that capture is automatic. Explicit syntax is now needed in the *opposite* case, **to opt out of capturing**: `impl Sized + use<>` or `use<T>`. Those are lifetime-adjacent annotations that vai must be able to spell.

## 3. What Rust 2024 and recent RFCs changed

| Change | Version | Effect on explicit lifetimes |
|---|---|---|
| RFC 141 elision | 1.0 | The three fn rules |
| RFC 599/1156 object defaults | 1.0 / 1.3 | `dyn Tr` default bounds |
| RFC 2115 `'_` + impl-header elision | 2018 (1.26-1.31) | `impl Foo<'_>`, `Thing<'_>`; in-band lifetimes **rejected** |
| RFC 2093 inferred outlives | 2018 | No `T: 'a` on structs |
| GATs | 1.65 | New mandatory `where Self: 'a` |
| RFC 3617 `use<..>` precise capturing | 1.82 (fns), 1.87 (traits/RPITIT) [R1.87] | Opt-out spelling for captures |
| RFC 3498 RPIT captures all in-scope lifetimes | **edition 2024** (1.85) [ED-2024] | Removes `+ 'a` / `Captures` from RPIT; `cargo fix` inserts `use<..>` to keep old semantics |
| `mismatched_lifetime_syntaxes` warn-by-default | 1.89 [LINT, R1.89] | Pushes toward visible `'_`; replaces `elided_named_lifetimes` |

[ED-2024] details:

```rust
fn f(_: &()) -> impl Sized {}
// 2021: impl Sized + use<>      2024: impl Sized + use<'_>
```

- Over-capturing has a cost for callers. A captured lifetime keeps the argument borrowed, so `fn test<'a>(x: &'a ()) -> impl Sized + 'static { capture(x) }` fails unless `capture` says `use<>`.
- `use<..>` can only list *named* parameters. An APIT (`impl Trait` in argument position) must first be turned into a named generic, which changes the turbofish API.
- Capturing `for<'a>` higher-ranked lifetimes in nested opaque types is not supported yet.
- The `impl_trait_overcaptures` migration lint rewrites code automatically where it can.

## 4. Implications for vai (bounding inference vs spelling)

- **Safe to infer (vai can omit; the lowering can emit elided Rust):** everything covered by rules 1-3, `dyn` defaults, `const`/`static` `'static`, impl-header `'_`, struct outlives bounds, `Fn`-sugar HRTBs and RPIT captures in 2024. To keep the lint quiet, vai should **emit `'_` on hidden path lifetimes** (2.9) even when the vai source omits them. That is a pure, local lowering rule.
- **Inference that would change semantics (not allowed):** choosing which input an output borrows from (2.2), `'static` versus generic outputs (2.3), and data-tied versus self-tied iterator outputs (2.5). Rust checks signatures without looking at bodies, so body-driven inference would make a vai signature mean something its Generated Rust signature does not say. A local heuristic is possible, e.g. "the only reference-typed field of `self`'s type", but it must lower to an *explicit* named lifetime in the Generated Rust.
- **Must be spellable in vai:** declaring a lifetime parameter on a type (2.1), output-to-input ties (2.2), `'static` (2.3, 2.7), `dyn Tr + 'a` (2.4), named impl lifetimes and associated types (2.5), GAT `where Self: 'a` (2.6), outlives bounds (2.7), `for<'a>` on non-`Fn` traits (2.8), and `use<..>` (2.10).
- **Best candidates for Sugar** (these lower 1:1 and need no new semantics):
  - one lifetime parameter per struct, auto-declared when there is a single borrowing field (`struct Parser { input: &str }` → `Parser<'a>`); with several, require names;
  - "borrowing from" annotations at the argument (`fn longest(x: &str, y: &str) -> &str from x, y`) lowering to a shared `'a`;
  - auto-generating the GAT `where Self: 'a` bound, which the compiler already demands mechanically.

## Open questions surfaced

1. Should vai's struct-lifetime sugar mirror the rejected in-band lifetimes idea from RFC 2115? It was rejected for readability, which is relevant prior art.
2. Which edition does Generated Rust target? Pinning 2024 gets the capture rules; mixed `.rs` crates on 2021 would interact through `use<..>`.
3. How do rustc lifetime diagnostics (E0106, E0495-style region errors) map back to vai source when vai elided something that the lowering made explicit? This connects to the diagnostics mapping in the pipeline decision.
4. Closure return-lifetime elision (2.8) is a Rust gap. vai cannot fix it without unstable `for<'a> |..|`; it needs a decision once closures are designed.
