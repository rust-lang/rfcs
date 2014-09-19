- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Change the address-of operator (`&`) to a borrow operator. This is an
alternative to #241 and #226 (cross-borrowing coercions). The borrow operator
would perform as many dereferences as possible and then take the address of the
result. The result of `&expr` would always have type `&T` where `T` does not
implement `Deref`.


# Motivation

In Rust the concept of ownership is more important than the precise level of
indirection. Whilst it is important to distinguish between values and references
for performance reasons, Rust's ownership model means it is less important to
know how many levels of indirection are involved in a reference.

It is annoying to have to write out `&*`, `&**`, etc. to convert from one
pointer kind to another. It is not really informative and just makes reading and
writing Rust more painful.

It would be nice to strongly enforce the principle that the first type a
programmer should think of for a function signature is `&T` and to discourage
use of types like `&Box<T>` or `Box<T>`, since these are less general. However,
that generality is somewhat lost if the user of such functions has to consider
how to convert to `&T`.


# Detailed design

Writing `&expr` has the effect of dereferencing `expr` as many times as possible
(by calling `deref` from the `Deref` trait or by doing a compiler-built-in
dereference) and taking the address of the result.

Where `T` is some type that does not implement `Deref`, `&x` will have type `&T`
if `x` has type `T`, `&T`, `Box<T>`, `Rc<T>`, `&Rc<T>`, `Box<&Rc<Box<&T>`, and
so forth.

`&mut expr` would behave the same way but take a mutable reference as the final
step. The expression would have type `&mut T`. The usual rules for dereferencing
and taking a mutable reference would apply, so the programmer cannot subvert
Rust's mutability invariants.

No coercions may be applied to `expr` in `&expr`, but they may be applied to
`&expr` if it would otherwise be possible.

Raw pointers would not be dereferenced by `&`. We expect raw pointer
dereferences to be explicit and to be in an unsafe block. So if `x` has type
`&Box<*Gc<T>>`, then `&x` would have type `&*Gc<T>`. Alternatively, we could
make attempting to dereference a raw pointer using `&` a type error, so `&x`
would give a type error and a note advising to use explicit dereferencing.

Writing `&(expr)` (and similarly for `&mut(expr)`) will have the effect of
taking the address of `expr` (the current semantics of `&expr`). If `expr` has
type `U`, for any `U`, then `&(expr)` will have type `&U`. This syntax is not
the greatest, and I'm very open to other suggestions. In particular writing
`&(some_big_expression)` will give the address-of not borrow behaviour, which
might be confusing. In practice, I hope this works, since when doing explicit
referencing/dereferencing, people often use brackets (e.g., `(&*x).foo()` would
become `(&(*x)).foo()`). I hope these cases are very rare. It is only necessary
when you need an expression to have type `&Rc<T>` or similar, and when that
expression is not the receiver of a method call.


# Drawbacks

Arguably, we should be very explicit about indirection in a systems language,
and this proposal blurs that distinctions somewhat.

When a function _does_ want to borrow an owning reference (e.g., takes a
`&Box<T>` or `&mut Vec<T>`), it would be more painful to call that function. I
believe this situation is rare, however.


# Alternatives

Take this proposal, but use a different operator. This new operator would have
the semantics proposed here for `&`, and `&` would continue to be an address-of
operator.

There are two RFCs for different flavours of cross-borrowing: #226 and #241.

#226 proposes sugaring `&*expr` as `expr` by doing a dereference and then an
address-of. This converts any pointer-like type to a borrowed reference.

#241 proposes sugaring `&*n expr` to `expr` where `*n` means any number of
dereferences. This converts any borrowed pointer-like type to a borrowed
reference, erasing multiple layers of indirection.

At a high level, #226 privileges the level of indirection, and #241 privileges
ownership. This RFC is closer to #241 in spirit, in that it erases multiple
layers of indirection and privileges ownership over indirection.

All three proposals mean less fiddling with `&` and `*` to get the type you want
and none of them erase the difference between a value and a reference (as auto-
borrowing would).

In many cases this proposal and #241 give similar results. The difference is
that this proposal is linked to an operator and is type independent, whereas
#241 is implicit and depends on the required type. An example which type checks
under #241, but not this proposal is:

```
fn foo(x: &Rc<T>) {
    let y: &T = x;
}
```

Under this proposal you would use `let y = &x;`.

I believe the advantages of this approach vs an implicit coercion are:

* better integration with type inference (note no explicit type in the above
  example);
* more easily predictable and explainable behaviour (because we always do
  as many dereferences as possible, c.f. a coercion which does _some_ number of
  dereferences, dependent on the expected type);
* does not complicate the coercion system, which is already fairly complex and
  obscure (RFC on this coming up soon, btw).

The principle advantage of the coercion approach is flexibility, in particular
in the case where we want to borrow a reference to a smart pointer, e.g.
(aturon),

```
fn wants_vec_ref(v: &mut Vec<u8>) { ... }

fn has_vec(v: Vec<u8>) {
    wants_vec_ref(&mut v); // coercing Vec to &mut Vec
}
```

Under this proposal `&mut v` would have type `&mut[u8]` so we would fail type
checking (I actually think this is desirable because it is more predictable,
although it is also a bit surprising). Instead you would write `&mut(v)`. (This
example assumes `Deref` for `Vec`, but the point stands without it, in general).


# Unresolved questions

Can we do better than `&(expr)` syntax for address-of?


## Slicing

There is a separate question about how to handle the `Vec<T>` -> `&[T]` and
`String` -> `&str` conversions. We currently support this conversion by calling
the `as_slice` method or using the empty slicing syntax (`expr[]`). If we want,
we could implement `Deref<[T]>` for `Vec<T>` and `Deref<str>` for `String`,
which would allow us to convert using `&*expr`. With this RFC, we could convert
using `&expr` (with RFC #226 the conversion would be implicit).

The question is really about `Vec`, `String`, and `Deref`, and is mostly
orthogonal to this RFC. As long as we accept this or one of the cross-borrowing
RFCs, then `Deref` could give us 'nice' conversions from `Vec` and `String`.
