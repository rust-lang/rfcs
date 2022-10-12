- Feature Name: `const_wildcard`
- Start Date: 2018-08-18
- RFC PR: [rust-lang/rfcs#2526](https://github.com/rust-lang/rfcs/pull/2526)
- Rust Issue: [rust-lang/rust#54912](https://github.com/rust-lang/rust/issues/54912)

# Summary
[summary]: #summary

Allow assigning constants to `_`, as in `const _: TYPE = VALUE`, analogous to
`let _ = VALUE`.

# Motivation
[motivation]: #motivation

The ability to ensure that code type checks while discarding the result is
useful, especially in custom derives. For example, the following code will not
compile if the type `MyType` doesn't implement the trait `MyTrait`:

```rust
const _FOO: () = {
    use std::marker::PhantomData;
    struct ImplementsMyTrait<T: MyTrait>(PhantomData<T>);
    let _ = ImplementsMyTrait::<MyType>(PhantomData); // type checking error if MyType: !MyTrait
    ()
};
```

Unfortunately, this requires coming up with a unique identifier to assign to.
This is error-prone because no matter what identifier is chosen, there's always
a possibility that a user will have already used the same identifier in their
code. If writing `const _: () = { ... }` were valid, then this would be a
non-issue - the `const _` could be repeated many times without conflicting with
any other identifier in scope.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Allow assigning to `_` when defining a new constant. Just like `let _`, this
doesn't introduce any new bindings, but still evaluates the rvalue at compile
time like any other constant.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following changes are made to the language:

## Grammar

The grammar of `item_const` is changed from:

```text
item_const : CONST ident ':' ty '=' expr ';' ;
```

to:

```text
item_const : CONST (ident | UNDERSCORE) ':' ty '=' expr ';' ;
```

## Type checking

When type checking an associated `const` item, the token `_` may not occur as
the name of the item.

When type checking a `const` item not inside an `impl` item, the token `_` is
permitted as the name of such an item. When that token does occur, it is
replaced with a freshly generated and unique identifier.

# Drawbacks
[drawbacks]: #drawbacks

The rules around constant identifiers are made somewhat more complicated, as is
the compiler logic for handling them. A distinction is introduced between
associated `const` items (inside `impl`s) and non-associated `const` items.

# Rationale and alternatives
[alternatives]: #alternatives

## Rationale

This would allow more ergonomic uses of a number of patterns used today:
- Ensuring that types have certain trait bounds in custom derives, as explained
  in the [Motivation] section.
- [`const_assert!`](https://docs.rs/static_assertions/0.2.5/static_assertions/macro.const_assert.html)
  and other macros in the
  [`static_assertions`](https://docs.rs/static_assertions/0.2.5/static_assertions/index.html)
  crate, which currently work only in a scope (so that they can use a `let`
  binding) or requires the user to specify a scope-unique name for a function
  which will be used to contain the expression that is the meat of the macro.

Eventually, we will likely want to support fully general pattern matching just
like in `let` bindings (e.g., `const (a, b): (u8, u8) = (1, 1)`) to not have
`const _` be a special case in the language. However, this RFC leaves the
details of such a design up to a future RFC.

## Alternatives

- We could provide procedural macros with an API that fetches a new,
  globally-unique identifier.
- We could support anonymous modules (`mod { ... }` or `mod _ { ... }`).
- We could support anonymous top-level functions (`fn _() { ... }`).

# Prior art
[prior-art]: #prior-art

Go allows unnamed constants using the syntax `const _ = ...`. It also allows
top-level variable bindings which are evaluated at init time, before `main` is
run - `var _ = ...`. This latter syntax is often used to ensure that a
particular type implements a particular interface, as in this example [from the
standard library](https://golang.org/src/math/big/ftoa.go#L379):

```go
var _ fmt.Formatter = &floatZero // *Float must implement fmt.Formatter
```

# Unresolved questions
[unresolved]: #unresolved-questions

None.
