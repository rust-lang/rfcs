- Start Date: 2015-2-3
- RFC PR: [rust-lang/rfcs#803](https://github.com/rust-lang/rfcs/pull/803)
- Rust Issue: [rust-lang/rust#23416](https://github.com/rust-lang/rust/issues/23416)
- Feature: `ascription`

# Summary

Add type ascription to expressions. (An earlier version of this RFC covered type
ascription in patterns too, that has been postponed).

See also discussion on [#354](https://github.com/rust-lang/rfcs/issues/354) and
[rust issue 10502](https://github.com/rust-lang/rust/issues/10502).


# Motivation

Type inference is imperfect. It is often useful to help type inference by
annotating a sub-expression with a type. Currently, this is only possible by
extracting the sub-expression into a variable using a `let` statement and/or
giving a type for a whole expression or pattern. This is un- ergonomic, and
sometimes impossible due to lifetime issues. Specifically, where a variable has
lifetime of its enclosing scope, but a sub-expression's lifetime is typically
limited to the nearest semi-colon.

The typical use case is where a function's return type is generic (e.g., collect).

Type ascription can also be used for documentation and debugging - where it is
unclear from the code which type will be inferred, type ascription can be used
to precisely communicate expectations to the compiler or other programmers.

By allowing type ascription in more places, we remove the inconsistency that
type ascription is currently only allowed on top-level patterns.

## Examples:

(Somewhat simplified examples, in these cases there are sometimes better
solutions with the current syntax).

Generic return type:

```
// Current.
let z = if ... {
    let x: Vec<_> = foo.enumerate().collect();
    x
} else {
    ...
};

// With type ascription.
let z = if ... {
    foo.enumerate().collect(): Vec<_>
} else {
    ...
};
```

Generic return type and coercion:

```
// Current.
let x: T = {
    let temp: U<_> = foo();
    temp
};

// With type ascription.
let x: T = foo(): U<_>;
```

# Detailed design

The syntax of expressions is extended with type ascription:

```
e ::= ... | e: T
```

where `e` is an expression and `T` is a type. Type ascription has the same
precedence as explicit coercions using `as`.

When type checking `e: T`, `e` must have the exact type `T`. Neither
subtyping nor coercions are permitted. `T` may be any well-formed
type. At runtime, type ascription is a no-op.

This feature should land behind the `ascription` feature gate.

### coercion vs `:`

In contrast to the `as` keyword, the `:` type ascription operator
equates the type of the expression it is applied to, rather than
applying coercions. One of the reasons for this is that many
expressions in Rust can serve two rules.  For example, consider a
variable reference like `path`.

- In the expression `&path`, `path` denotes the address of the local variable
  `path`.
- In the expression `f(path)`, the value of the local variable is being used.

So one must then decide whether to treat `path: T` as an lvalue or an rvalue.
If we permit coercions or subtyping with type ascription, neither choice is
satisfactory.

**Treating ascription as lvalues.** If we said that `path: T` is an
lvalue, that introduces the potential for unsoundness unless we know
that `typeof(path) == T`. Consider this example, where we apply the
`&mut` operator to a coercion `foo: T` (here, we imagine both that `S
<: T` and the `:` operator permitted subtyping):

```
let mut foo: S = ...;
{
    let bar = &mut (foo: T);  // S <: T, no coercion required
    *bar = ... : T;
}
// Whoops, foo has type T, but the compiler thinks it has type S, where potentially T </: S
```

**Treat ascription as rvalues.** If we treat ascription expressions as
rvalues (i.e., create a temporary in lvalue position), then we don't
have the soundness problem, but we do get the unexpected result that
`&(x: T)` is not in fact a reference to `x`, but a reference to a
temporary copy of `x`.

An earlier draft of this RFC proposed a compromise, where type
ascription expressions inherit their 'lvalue-ness' from their
underlying expressions, but the semantics of an ascription varies
depending on its context. In particular, in a reference context, type
equality was used, but otherwise coercions and subtyping
were permitted. However, this rule was later jduged to be too subtle.
It is hard to explain and annoying to implement.

Therefore, the RFC was amended to simply use type equality all the
time, sidestepping the problem. This still preserves the major use
case for type ascription, which is annotating the return type of a
function in an ergonomic way.

# Drawbacks

More syntax, another feature in the language.

Interacts poorly with struct initialisers (changing the syntax for struct
literals has been [discussed and rejected](https://github.com/rust-lang/rfcs/pull/65)
and again in [discuss](http://internals.rust-lang.org/t/replace-point-x-3-y-5-with-point-x-3-y-5/198)).

If we introduce named arguments in the future, then it would make it more
difficult to support the same syntax as field initialisers.


# Alternatives

We could do nothing and force programmers to use temporary variables to specify
a type. However, this is less ergonomic and has problems with scopes/lifetimes.

Rely on explicit coercions - the current plan [RFC 401](https://github.com/rust-lang/rfcs/blob/master/text/0401-coercions.md)
is to allow explicit coercion to any valid type and to use a customisable lint
for trivial casts (that is, those given by subtyping, including the identity
case). If we allow trivial casts, then we could always use explicit coercions
instead of type ascription. However, we would then lose the distinction between
implicit coercions which are safe and explicit coercions, such as narrowing,
which require more programmer attention. This also does not help with patterns.

We could use a different symbol or keyword instead of `:`, e.g., `is`.


# Unresolved questions

Is the suggested precedence correct?

Should we remove integer suffixes in favour of type ascription?

Style guidelines - should we recommend spacing or parenthesis to make type
ascription syntax more easily recognisable?
