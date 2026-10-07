- Feature Name: `unsafe_global_asm`
- Start Date: 2026-10-06
- RFC PR: [rust-lang/rfcs#4014](https://github.com/rust-lang/rfcs/pull/4014)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Require `unsafe` for usage of the built-in `global_asm` macro.

## Motivation
[motivation]: #motivation

The `global_asm` macro can be used to cause undefined behaviour by overwriting symbols.
The use of this macro is currently not marked as unsafe in any way.
New syntax is required to close this unsoundness, because it is currently not possible to mark a macro
invocation as unsafe.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When declaring a function in a `global_asm` macro like this

```rust
global_asm!("
.globl write
write:
ret
");
```
it will cause Rust to generate a globally visible function with the linker name `write`.
Code that wants to call the POSIX `write` function might call this one instead.
This can lead to UB if:
- the assembly does not follow the ABI of the `write` function correctly.
- the assembly behaves in a way that is incompatible with the contract of the `write` function.

To avoid this `global_asm` must not declare functions or globals whose names clash with other functions or globals.
Since the compiler in general cannot check this it must be done by the programmer.
The syntax for discharging the safety obligation is the same as the syntax for unsafe trait implementations, where the `unsafe`
is also used to mark the safety obligations.

```rust
/// SAFETY: there is no other global function named `my_own_write`. No other symbols are declared.
unsafe global_asm!("
.globl my_own_write
my_own_write:
ret
");
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `global_asm` macro is considered unsafe and (starting from the next edition) can only be invoked by
using the `unsafe` keyword.
Other macros can't be invoked in this way.

```rust
unsafe global_asm!(""); // ERROR now
global_asm!(""); // ERROR on next edition
unsafe my_macro!(); // ERROR now and on next edition
```

To keep backwards compatibility using the `global_asm` macro without `unsafe` on older editions is not
a hard error, but is linted against.

The (currently unimplemented) feature [`global_asm_in_statement_position`](https://github.com/rust-lang/rust/issues/156965)
allows using `global_asm` in more positions. It will be adjusted to follow the same syntax laid out here.
The `unsafe` keyword is required, even if `global_asm` is invoked in an unsafe block.
```rust
fn outer() {
  unsafe {
    global_asm!(""); // ERROR
  }
}
```

Because `global_asm` is an item and not a statement this behaviour is the same as other items (unsafe trait
implementations and functions with unsafe attributes).

The syntax of [MacroItem](https://doc.rust-lang.org/nightly/reference/items.html#grammar-MacroItem) is changed to the following:
```grammar,items
MacroItem ->
      `unsafe`? MacroInvocationSemi
    | MacroRulesDefinition
```

Using `unsafe` for macros in non-item positions is still disallowed in the grammar.

## Drawbacks
[drawbacks]: #drawbacks

- Disallowing the old syntax is a breaking change.
- It makes `global_asm` more special since users can't require the use of `unsafe` for their own macros.
- If unsafe macros would ever be allowed the syntax should be consistent, so this limits the possibilities.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- **Do Nothing**: This unsoundness stays open.
  As [RFC 3324](https://github.com/rust-lang/rfcs/blob/master/text/3325-unsafe-attributes.md) argues, having the `unsafe_code` lint warn against this is not enough,
  because the lint is opt-in, while Rust has "safety by default".
- **Disallow the old syntax on all editions**: This would break all rust code that uses `global_asm`.
- **Other syntax**: There are a couple of alternatives.
  - It could be renamed to something like `unsafe_global_asm`.
  - It could require `unsafe` as part of its syntax (`global_asm!(unsafe {".."})` or `global_asm!(unsafe "..")`)
  - Unsafe blocks in item position could be added.
  - It could require an `#[unsafe]` or `#[unsafe()]` attribute. This syntax implies maybe even more generality, because one could expect `#[unsafe] impl Send for MyType {}` to work.

The chosen syntax mirrors the one chosen for unsafe attributes in that it does not rename the problematic
item and in that it defines a syntax for unsafe macros.

## Prior art
[prior-art]: #prior-art

Especially [RFC 3324](https://github.com/rust-lang/rfcs/blob/master/text/3325-unsafe-attributes.md), which made `no_mangle`
and similar attributes require `unsafe` is very similar to this RFC.
In both cases
- the unsoundness is related to symbols of items.
- the problematic feature was stabilised a long time ago.
- new syntax had to be added to mark the unsafety.
- the new syntax could be extended to unsafe macros in the future.
- the old syntax is only disallowed on a new edition.

The `unsafe_code` lint currently already fires on uses of `global_asm` with the note:
"using this macro is unsafe even though it does not need an `unsafe` block".
Something that is considered unsafe, but does not require an unsafe annotation to use is not a useful
concept. That `unsafe` marks *every* part of the code that requires special care is a big part of rusts safety story.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- While this syntax seems the most sensible to me i am not opposed to another syntax.
- **Rollout of the lint / if this should be done across all editions**:
  I don't see much advantage of breaking this code on older editions.

## Future possibilities
[future-possibilities]: #future-possibilities

- Users could be allowed to require `unsafe` for their own macros, which would reuse this syntax.
- After that it could be allowed to use `unsafe` for more macros, not just item macros.
