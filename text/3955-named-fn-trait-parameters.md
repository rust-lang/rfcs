- Feature Name: `named_fn_trait_parameters`
- Start Date: 2026-04-24
- RFC PR: [rust-lang/rfcs#3955](https://github.com/rust-lang/rfcs/pull/3955)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Allow (optional) named function parameters in parenthesized generic argument lists, such as those of `Fn`, `FnMut`, `FnOnce`, `AsyncFn`, `AsyncFnMut`, and `AsyncFnOnce`.
For example:
```rust
fn parse_my_data(
    data: &str,
    log: impl Fn(msg: String, priority: usize)
) { }
```
Similar to named function pointer parameters, these names don't affect rust's semantics.

## Motivation
[motivation]: #motivation

### Benefit: Better documentation

This allows users to better document the meaning of parameters in signatures. This is the primary benefit of this RFC.

For example, it is not immediately clear what the `String` and `usize` refer to in the type of `log`, providing names like in the example above is much clearer.

```rust
fn parse_my_data(
    data: &str,
    log: impl Fn(String, usize)
) { }
```

The parameter names should also show up on rustdoc.

### Benefit: Better LSP hints

When calling `log` in the body of `parse_my_data`, the LSP can provide the function parameter names as "inlay parameter name hints":
log(`data: `"Message".to_string(), `priority: `1);

This is a concrete advantage of this approach over using comments to do the same thing, such as in:
```rust
fn parse_my_data(
    data: &str,
    log: impl Fn(/* msg */ String, /* priority */ usize)
) { }
```

### Benefit: Better consistency with `fn` pointers

Imagine if `parse_my_data` looked like this:
```rust
fn parse_my_data(
    data: &str,
    log: fn(msg: String, priority: usize)
) { }
```

If due to new requirements the user decides that `impl Fn` suits the usecase better, having to remove the parameter names is unintuive.
This RFC removes this problem.

Note that the syntax for this feature does not exactly match that of `fn` pointers, see the [reference level explanation](#reference-level-explanation).

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can give names to parameters to the `Fn` trait and its friends to better document the meaning of these parameters, to help people who call your function.
These names are optional and don't have any semantic meaning. Named and unnamed parameters can be mixed, for example:

```rust
fn parse_my_data(
    data: &str,
    log: impl Fn(String, priority: usize)
) { }
```

This same syntax also applies to trait bounds, for example:
```rust
fn parse_my_data<
    L: Fn(msg: String, priority: usize)
>(
    data: &str,
    log: L
) { }
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Before this RFC, the syntax rules of parenthesized generic argument lists are:
```grammar,types
TypePathFn -> `(` TypePathFnInputs? `)` (`->` TypeNoBounds)?

TypePathFnInputs -> Type (`,` Type)* `,`?
```

After this RFC, the `TypePathFnInputs` rule will be replaced by:

```grammar,types
TypePathFnInputs -> TypePathFnInput (`,` TypePathFnInput)* `,`?

TypePathFnInput -> ( ( IDENTIFIER | `_` ) `:` )? Type
```

Note that this means that:

- Attributes are not allowed on parameters of these traits. This remains unchanged from the current situation. The following will not work:
  ```rust
  fn test(x: impl Fn(#[cfg(...)] msg: String, priority: usize), y: usize) { }
  ```
  Note that attributes are already allowed on `fn` pointers:
  ```rust
  fn test(x: fn(#[cfg(...)] msg: String, priority: usize), y: usize) { }
  ```
  The reason why attributes are not allowed is to keep this RFC and the implementation simple. 
  Allowing attributes such as `#[cfg(...)]` could be useful, but is out of scope for this RFC.
  
- This syntax does not match that of `fn` pointers exactly.
  For historic reasons, the following `fn` pointer type is allowed:
  ```rust
  #[cfg(false)]
  type T = fn(mut x: (), &x: (), &&x: (), false: (), &_: (), &true: ());
  ```
  But this RFC proposes that the following `impl Fn` type is not allowed:

  ```rust
  #[cfg(false)]
  impl Fn(mut x: (), &x: (), &&x: (), false: (), &_: (), &true: ());
  ```

  The names of function parameters are limited to ``IDENTIFIER | `_` ``.
  The reason why we don't match the syntax of `fn` pointer types is because the syntax was a historic mistake and we should not repeat that.

## Drawbacks
[drawbacks]: #drawbacks

* This makes the syntax of `impl Fn` and friends slightly more complicated
* This keeps the syntax of `impl Fn` and friends inconsistent with that of `fn` pointers, as to avoid the historic mistakes mentioned in the [reference level explanation](#reference-level-explanation).

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* An alternative would be to match the `fn` pointer syntax perfectly. This would make the implementation more complicated, without much benefit other than consistency.
* This needs to be implemented in the language, it cannot be provided by a macro or library as it affects syntactic sugar of the language itself.
* This makes Rust code easier to read, as it adds better ways to document function signatures.

## Prior art
[prior-art]: #prior-art

In Rust, this is already allowed in `fn` pointers:
```rust
type LogFunction = fn(msg: String, priority: usize);
```

In TypeScript:
```ts
type LogFunction = (msg: string, priority: number) => void;
```

In Kotlin:
```kotlin
fun log(data: String, logFunction: (msg: String, priority: Int) -> Unit) { }
```

## Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should duplicate parameter names be allowed in named fn trait arguments? This is currently allowed for `fn` pointers and other functions without an accompanying `Body`.
  ```rust
  type T = fn(x: usize, x: usize);
  ```
  ```rust
  trait Test {
    fn thing(x: usize, x: usize);
  }
  ```

## Future possibilities
[future-possibilities]: #future-possibilities

* We could allow attributes on `impl Fn` parameters in the future.

