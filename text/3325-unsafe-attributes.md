- Feature Name: `unsafe_attributes`
- Start Date: 2022-10-11
- RFC PR: [rust-lang/rfcs#3325](https://github.com/rust-lang/rfcs/pull/3325)
- Tracking Issue: [rust-lang/rust#123757](https://github.com/rust-lang/rust/issues/123757)

# Summary
[summary]: #summary

Consider some attributes 'unsafe', so that they must only be used like this:

```rust
#[unsafe(no_mangle)]
```

# Motivation
[motivation]: #motivation

Some of our attributes, such as `no_mangle`, can be used to
[cause Undefined Behavior without any `unsafe` block](https://github.com/rust-lang/rust/issues/28179).
If this was regular code we would require them to be placed in an `unsafe {}`
block, but since they are attributes that makes less sense. Hence we need a
concept of 'unsafe attributes' and accompanying syntax to declare that one is
aware of the UB risks here (and it might be good to add a SAFETY comment
explaining why this use of the attribute is fine).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

*Example explanation for `no_mangle`; the other attributes need something similar.*

When declaring a function like this

```rust
#[no_mangle]
pub fn write(...) { ... }
```

this will cause Rust to generate a globally visible function with the
linker/export name `write`. As consequence of that, other code that wants to
call the
[POSIX `write` function](https://pubs.opengroup.org/onlinepubs/9699919799/functions/write.html) might
end up calling this other `write` instead. This can easily lead to Undefined
Behavior:
- The other `write` might have the wrong signature, so arguments are passed
  incorrectly.
- The other `write` might not have the expected behavior of
  [write](https://man7.org/linux/man-pages/man2/write.2.html), causing code
  relying on this behavior to misbehave.

To avoid this, when declaring a function `no_mangle`, it is important that the
name of the function does not clash with other globally named functions. Similar
to how `unsafe { ... }` blocks are used to acknowledge that this code is
dangerous and needs manual checking, `unsafe(no_mangle)` acknowledges that
`no_mangle` is dangerous and needs to be manually checked for correctness:

```rust
// SAFETY: there is no other global function of this name
#[unsafe(no_mangle)]
pub fn my_own_write(...) { ... }
```

Note that when writing a library crate, it is in general not possible to make
claims like "there is no other global function of this name". This is a
fundamental limitation of the global linking namespace, and not something Rust
currently is able to overcome. Libraries that make such assumptions should
ideally document somewhere publicly that they consider some namespace, i.e.
every function starting with `_mycrate__`, to be reserved for their exclusive
use.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Some attributes (e.g. `no_mangle`, `export_name`, `link_section` -- see
[here](https://github.com/rust-lang/rust/issues/82499) for a more complete list)
are considered "unsafe" attributes. An unsafe attribute must only be used inside
`unsafe(...)` in the attribute declaration, like

```rust
#[unsafe(no_mangle)]
```

For backwards compatibility reasons, using these attributes outside of
`unsafe(...)` is just a lint, not a hard error. The lint is called
`unsafe_attr_outside_unsafe`. Initially, this lint will be allow-by-default.
Unsafe attributes that are added in the future can hard-require `unsafe` from
the start since the backwards compatibility concern does not apply to them.
The 2024 edition is also expected to increase the severity of this lint,
possibly even making it a hard error.

Syntactically, for each unsafe attribute `attr`, we now also accept
`unsafe(attr)` anywhere that `attr` can be used (in particular, inside
`cfg_attr`). `unsafe` cannot be nested, cannot contain `cfg_attr`, and cannot
contain any other (non-unsafe) attributes. Only a single attribute can be used
inside `unsafe`, i.e., `unsafe(foo, bar)` is invalid.

The `deny(unsafe_code)` lint denies the use of unsafe attributes both inside and
outside of `unsafe(...)` blocks. (That lint currently has special handling to
deny these attributes. Once there is a general notion of 'unsafe attributes' as
proposed by this RFC, that special handling should no longer be needed.)

The `unsafe(...)` attribute block is required even for functions declared inside
an `unsafe` block. That is, the following is an error:

```rust
fn outer() {
  unsafe {
    #[no_mangle]
    fn write() {}
  }
}
```

This matches the fact that expression-level unsafety is not inherited for items
declared inside other items.

# Drawbacks
[drawbacks]: #drawbacks

I think if we had thought of this around Rust 1.0, then this would be rather
uncontroversial. As things stand now, this proposal will cause a lot of churn
since all existing uses of these unsafe attributes need to be adjusted. The
warning for using unsafe attributes outside `unsafe(...)` should probably have
an auto-fix available to help ease the transition here.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- **Nothing.** We could do nothing at all, and live with the status quo. However
  then we will not be able to fix issues like
  [`no_mangle` being unsound](https://github.com/rust-lang/rust/issues/28179),
  which is one of the oldest open soundness issues.
- **Rename.** We could just rename the attributes to `unsafe_no_mangle` etc.
  However that is inconsistent with how we approach `unsafe` on expressions, and
  feels much less systematic and much more ad-hoc.
- **`deny(unsafe_code)`.** We already
  [started the process](https://github.com/rust-lang/rust/issues/82499) of
  rejecting these attributes when `deny(unsafe_code)` is used. We could say that
  is enough. However the RFC authors thinks that is insufficient, since only few
  crates use that lint, and since it is the wrong default for Rust (users have
  to opt-in to a soundness-critical diagnostic -- that's totally against the
  "safety by default" goal of Rust). This RFC says that yes, `deny(unsafe_code)`
  should deny those attributes, but we should go further and require an explicit
  `unsafe(...)` attribute block for them to be used at all.
- **Item-level unsafe blocks.** We could find some way to have 'unsafe blocks'
  around entire functions or modules. However, those would go against the usual
  goal of keeping `unsafe` blocks small. Big `unsafe` blocks risk accidentally
  calling an unsafe operation in there without even realizing it.
- **Other syntax.** Obviously we could pick a different syntax for the same
  concept, but this seems like the most natural marriage of the idea of unsafe
  blocks from regular code, and the existing attributes syntax.

# Prior art
[prior-art]: #prior-art

We have `unsafe` blocks; this is basically the same thing for the "attributes
DSL".

In the attribute DSL, we already have a "nesting" construct: `cfg_attr`. That
allows terms like
`#[cfg_attr(debug_assertions, deny(unsafe_code), allow(unused))]`, so there is
precedent for having a list of attributes inside a single attribute.

I don't know of other languages that would distinguish safe and unsafe
attributes.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- **Different lint staging.** The lint on using existing unsafe attributes like
  `no_mangle` outside `unsafe(...)` could be staged in various ways: it could be
  warn-by-default to start or we wait a while before to do that, it could be
  edition-dependent, it might eventually be deny-by-default or even a hard error
  on some editions -- there are lots of details here, which can be determined
  later during the process.

# Future possibilities
[future-possibilities]: #future-possibilities

- **Unsafe attribute proc macros.** We could imagine something like
    ```
    #[proc_macro_attribute(require_unsafe)]
    fn spoopy(args: TokenStream, input: TokenStream) -> TokenStream {…}
    ```
  to declare that an attribute proc macro is unsafe to use, and must only
  occur as an unsafe macro. Such an unsafe-to-use attribute proc macro must
  declare in a comment what its safety requirements are. (This is the `unsafe`
  from `unsafe fn`, whereas the rest of the RFC is using the `unsafe` from
  `unsafe { ... }`.)
- **Unsafe derive.** We could use `#[unsafe(derive(Trait))]` to derive an
  `unsafe impl` where the deriving macro itself cannot check all required safety
  conditions (i.e., this is 'unsafe to derive').
- **Unsafe tool attributes.** Same as above, but for tool attributes.
- **Unsafe attributes on statements.** For now, the only unsafe attributes we
  have don't make sense on the statement level. Once we do have unsafe statement
  attributes, we need to figure out whether inside `unsafe {}` blocks one still
  needs to also write `unsafe(...)`.
- **Lists and nesting.** We could specify that `unsafe(...)` may contain a list
  of arbitrary attributes (including safe ones), may be nested, and may contain
  `cfg_attr` that gets expanded appropriately. However that could make it tricky
  to consistently support non-builtin unsafe attributes in the future, so the
  RFC proposes to not do that yet. The current approach is forward-compatible
  with allowing lists and nesting in the future.
- **Unsafe crates.** Some attributes' requirements cannot be fully discharged
  locally. For instance, if a lib crate uses `no_mangle`, this really puts a
  burden on *the author of the final binary* to ensure that the symbol dos not
  conflict. In the future it would be better if rust tooling could automatically
  surface a such requirements to downstream code, for example by an automatic
  "unsafe attributes used" listing in a crate's generated rustdoc.
