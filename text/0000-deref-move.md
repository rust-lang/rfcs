- Feature Name: `deref_move`
- Start Date: 2018-05-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a new `DerefMove` trait that allows consuming a smart pointer to move its
contents, as in `let x = *p;`.

For more background and discussion, see #178 and #997.

# Motivation
[motivation]: #motivation

Currently, two smart pointer traits provide access to values contained in smart
pointers: `Deref` and `DerefMut`. These traits allow overloading of the
dereferencing (unary `*`) operator, and by extension participate in autoderef in
a number of places, such as method calls and index expressions.

These two traits, however, only dereference references to produce references to
the contained value. They do not offer the option to dereference by value,
consuming the smart pointer to produce the owned value inside. As a special
case, `Box<T>` has compiler support allowing it to be dereferenced (explicitly
or implicitly) to produce the inner `T`. Because this is special-cased in the
compiler, this functionality is not available for other smart pointer types.

Other smart pointers may support this consuming operation, but it must be done
explicitly due to the lack of language support. For instance, in the stdlib, the
following methods both consume a smart pointer to produce the inner value:

* `ManuallyDrop<T>::into_inner()`
* `binary_heap::PeakMut<'a, T>::pop()`
* `Cow<'a, B>::into_owned()`

I have not made any attempt to survey into the larger ecosystem for more
examples, but there are probably many more.

Note that removing `FnBox` is explicitly *not* part of the motivation for this
RFC. It has been suggested in a few places, such as in discussion of
rust-lang/rust#28796, that `DerefMove` would allow `Box<FnOnce()>` to be called
without needing the `FnBox` workaround. This is not the case, however: the inner
value is dynamically-sized and thus cannot be moved onto the stack. Indeed, one
can call a `Box<T>` where `T` is a concrete type implementing `FnOnce()`.
`DerefMove` would allow `Box<FnOnce()>` to continue not working, just without a
special case in the compiler to allow it to do so.

This will not remove all of `Box`'s special cases from the compiler, but
represents progress towards that goal.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The following trait is defined in `std::ops`:

```rust
trait DerefMove : DerefMut {
    fn deref_move(self) -> Self::Target;
}
```

This trait allows moving out of the result of a derefence. This can happen
either explicitly, as in `let x = *p;`, or implicitly, as in `p.into_foo()`.
`Box<T>` from the standard library implements `DerefMove`, allowing you to move
out of it.

```rust
let b = Box::new("s".to_owned());
let s = *b;
// ERROR: b has already been moved on the previous line.
// let t = *b;
```

If the `Target` type is `Copy`, then `DerefMove` will not be used, and instead
`Deref` will be used and the resulting referenced value will be copied:

```rust
let b = Box::new(0i32);
// Since i32 is Copy, this is equivalent to i = b.deref().
let i = *b;
// That makes this legal, since b was not moved.
let j = *b;
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`DerefMove` is a lang item that, like `Deref` and `DerefMut`, plays a role in
dereferencing, both explicitly with the unary `*` operator and implicitly with
autoderef.

`deref_move` is only called in contexts where a value is needed to be moved out.
This can occur either because an explicit dereference is used in a context where
a value is to be moved, or because autoderef selects a method taking `self` (or
`Box<self>`) as a parameter. In both cases, when evaluating whether to call
`deref_move`, the compiler will look at whether `Target` implements `Copy`. If
it does not, then `deref_move` is called to move the target out of the smart
pointer. If it does, then instead the compiler will only call `deref` and then
copy out of the returned reference. For generic types, the compiler only checks
if a `Copy` bound is available at the use site; it is not done during
monomorphization.

For explicit dereference, the changes are straightforward. Any explicit
dereference of a `DerefMove` type becomes a movable expression, just as
dereference of `Box` is today.

For autodereference: 

* In a function call expression, the callee operand may be dereferenced and
  moved if the `Target` implements `FnOnce` and no reference-based call trait.
* In a method call expression, the receiver operand may be dereferenced and
  moved if the function selected by overload resolution takes `self` or
  `Box<self>` as a parameter (in the latter case, this would require a smart
  pointer containing a `Box<Self>`, of course).
* In a field access expression, the left-hand operand may be dereferenced and
  moved if the result of the expression is moved. In this case, the move is
  always entire; partial move is not supported when a dereference is necessary.
* An index expression can never move its left operand, so it is unaffected.
* A borrow expression can never move its operand, so it is unaffected.

No change is made to overload resolution of method calls. This means that calls
which are invalid today due to preferring `T` over `&T` may now be legal,
selecting to move out of the smart pointer rather than giving an error.

There is an ambiguity in a method call expression when the `Target` type of the
receiver is a reference, and that reference type has a method taking `self`. In
this case, autodereference could either dereference via `deref_move` or via
`deref` or `defer_mut`. In this case, it will prefer the latter (for `deref`, it
is a special case of the `Copy` rule above, but for `deref_mut` it is not) to
avoid unnecessarily moving out of the receiver.

# Drawbacks
[drawbacks]: #drawbacks

## Nobody understands autoderef already and this makes it worse

Or at least, it would if `Box` wasn't already complicating it in the same way.

## Copy behaviour is unintuitive in generic cases

If a type has nontrivial/side-effectful `defer_move`, it may be slightly
confusing why it is not used for `Copy` types in non-generic code, but is used
in generic code.

## Breaking change to an edge case of `Box<&mut T>`

Currently, a `Box<&mut T>` variable [can be
dereferenced](https://play.rust-lang.org/?gist=5b3f046ecf29da8f00fde3bd1d2a3df4&version=nightly&mode=debug)
in at least some cases into a `&mut T` without either requiring that the
variable be mutable (as would be the case for any other `DerefMut` type), or
moving out of the `Box`. This behaviour would not be maintained if its magical
dereferencing were replaced with `DerefMove`, as `DerefMut` requires a mutable
variable.

This is an edge case, as it requires `Box<&mut T>`, which is an unlikely type,
and in most cases it can be trivially fixed by making the variable mutable. It's
possible there are more complex cases (such as if the `Box` itself is stored in
another smart pointer) where this is not easily fixed, but I would be
unsurprised if there were few, if any, instances of this in extant code.
Moreover, the behaviour is arguably a bug because it is taking a mutable borrow
of an immutable variable (and one can modify the above example to get an error
message implying exactly that), though the semantics are more like the unique
immutable borrows used by closures.

As a result, I propose that this breaking change be accepted after verifying its
effect on Cargo packages.

# Rationale and alternatives
[alternatives]: #alternatives

## Copy handling

The handling of copies is the real complexity of `DerefMove`. Currently, one can
copy out of a `Box` without consuming it, so this desirable behaviour needs to
be preserved. The proposal is to do so by special casing in the compiler.
However, it could also be done via, for instance, a separate method in the Deref
trait definition:

```rust
   fn deref_copy(&self) -> Self::Target where Self::Target : Copy {
       return self.deref();
   }
```

This still requires special-casing in the compiler, but it is less magical. One
could even envision a `DerefCopy` trait with similar purpose.

These approaches would, however, make diverging implementations (that is,
implementations for which the behaviour of the various deref traits is
unintuitive because they don't actually refer to the same underlying value) more
difficult to write, and we want to avoid that. And they would not fundamentally
solve the special casing in the compiler; to do that, we would really need a
more advanced type system allowing us to condition the type of `self` in
`deref_move` on whether or not `Target : Copy`.

With negative trait bounds, we could also bound `DerefMove` on `Target : !Copy`,
eliminating the problem, at the cost of requiring every implementation to also
bound on `!Copy`.

## Status quo

The status quo is disfavoured because it requires special-casing `Box` in the
compiler, privileging it over other smart pointers. While there's no obvious
call in the stdlib for `DerefMut` other than `Box` (see below), it prevents
other library authors from writing similar code.

## `IndexMove` trait

One option considered in the early discussion was an `IndexMove` trait, similar
to `DerefMove`, which would consume a collection and return a single element. I
have not included this in this RFC because no compelling use case was presented,
and the behaviour of `let s = v[0];` actually consuming `v` seems likely to add
cognitive overhead to the language.

Compared to other methods of moving out of a collection, `IndexMove` may provide
slight micro-optimization in some cases, but it's not clear without benchmarks
that this is actually meaningfully better than using, say,
`.into_inter().nth(n)`. Given that the `IndexMove` was proposed largely from
parallel construction from `DerefMove` without any specific use cases, it does
not seem worthwhile to move further with it at this time.

## Weakening `DerefMut` bound

The `DerefMut` bound on `DerefMove` and, in particular, the `Deref` bound that
it implies, constrain implementors somewhat. It requires that `Deref` be
implementable, so the implementation is not allowed to overload dereferencing in
such a way that it produces a new value each time. This would not be useful for
non-`Copy` types, since they would be consumed, but a `Copy` type could in this
fashion allow itself to be copied and then produce a new value on each
derefence. This would be useful to avoid redefining methods on the outer type.

This very much goes against the intent of the `Deref` traits to be restricted
smart pointers, however. `Deref` is currently the only way for methods of one
type to be callable on another, but it is deliberately limited in scope. For
instance, one might be tempted to implement `Deref` on a newtype for this
purpose, but this is considered unidiomatic. Additionally, this is a bandaid for
the fact that newtypes would also often like to be able to inherent trait
implementations, and inhereting methods is not sufficient for that.

The additional requirement is to avoid complication of the following case:

```rust
struct RefMethod();

trait Consume : Sized {
    fn consume(self) {}
}

impl<'a> Consume for &'a mut RefMethod {}

struct BadDeref<'a> (&'a mut RefMethod);

impl<'a> Deref for BadDeref<'a> {
    type Target = RefMethod;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<'a> DerefMove for BadDeref<'a> {
    fn deref_move(self) -> Self::Target { self.0 }
}

fn main() {
    let mut r = RefMethod{};
    let b = BadDeref(&mut r);
    b.consume();
    b.consume();
}
```

In this case, the first `b.consume()` is legal, but the second `b.consume()` is
not since `b` was already moved out of. But if `BadDeref` also implements
`DerefMut`, then the first `b.consume()` becomes illegal because `deref_mut`
becomes preferred and `b` is immutable. By requiring that `DerefMove :
DerefMut`, this edge case cannot be encountered: this code will always be
illegal unless `b` is made mutable.

# Prior art
[prior-art]: #prior-art

The compiler special case for `Box` is the most compelling prior art. This RFC
is written with that behaviour in mind; one of the most important criteria here
is that `Box` should remain completely backwards-compatible. In case of any
conflict between current `Box` behaviour and the behaviour specified in this
RFC that isn't explicitly called out as being different, it should be considered
a bug in the RFC and either this RFC should be corrected or more discussion
should ensue.

# Unresolved questions
[unresolved]: #unresolved-questions

## Implementors

It's not clear to me which, if any, of the smart pointers above should implement
`DerefMove` other than `Box`. It requires discussion of whether inadvertently
moving out of the pointer could cause issues. I think that it's reasonable to
start with only `Box` for now:

* `Cow` does not implement `DerefMut`, so it is ineligible.
* `PeekMut`'s `pop` has side effects on the containing collection, and so it
  seems best to require it to be moved explicitly.
* Moving out of a `ManualDrop` causes the newly-moved value to regain normal
  drop semantics. This seems like the least dangerous, given the mechanics of
  `ManualDrop`, but it still probably deserves separate consideration.

Additional implementations could, of course, be done as separate proposals for
the libs team.

## Preferring to copy the pointer

Some earlier discussion proposed that when a type implements both `DerefMove`
and `Copy`, then the special case of a `Copy` target does not apply. In other
words, given `let x = *p;`, if both `p` and `p`'s target type are `Copy`, then
rather than calling `deref` and copying the result, this assignment would call
`deref_mut`, copying `p` to use as the receiver.

Because of the above decision to require `DerefMut`, `p` must also be able to
get a mutable reference to its target. If it were `Copy`, then, it almost
certainly owns its target, meaning that copying `p` is going to copy `*p`
anyway. So copying only `*p` is a lesser copy, and simplifies the rules by
eliminating a special case. But we may decide that we want to make `DerefMove` a
bit more general, in which case we might want to consider this behaviour.
