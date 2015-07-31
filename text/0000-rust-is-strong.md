- Feature Name: rust-is-strong
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Abandon support for Weak on Rc and Arc in favour of external crates that
provide the functionality. This resolves some open API questions, while making
them genuinely "pay for what you use".

# Motivation

Weak reference-counted pointers have long been a bit of an awkward issue for
the libs team. As a refresher, this is the current Rc/Arc design:

Creating a new `Rc<T>` allocates an `RcBox<T>` which stores all the actual
Rc-meats. The layout of these structs is as follows:

```rust
struct Rc<T> {
    ptr: *const RcBox<T>,
}

struct RcBox<T> {
    strong_count: Cell<usize>,
    weak_count: Cell<usize>,
    data: T,
}
```

There's some slightly complex logic for the two counts that I think was stolen
from boost, but for simplicity I'll assume they store the actual counts.

Rcs frob the strong_count, while Weaks frob the weak_count. There are 4
interesting cases when dropping an Rc or Weak:

* Only weak hits 0: do nothing
* Only strong hits 0: drop `data` but *don't* free the RcBox
* Strong hits 0 while weak was already 0: drop and free
* Weak hits 0 while strong was already 0: free

The primary motivation for this design is cycles: If you have a cycle of
Rcs, it will be impossible to actually free it, because each element keeps
the next element alive, ultimately keeping itself alive.

However if you break a cycle with a Weak, one of the nodes will have its
contents dropped, which will in turn cause the next node to be *completely
freed*, cascading all the way around the cycle until the weakly-pointed-to
node finally becomes properly freed.

An example:

```text
- is strong link
~ is weak link
@ is dropped
$ is dropped + freed (freeing is cash-money)

    owner (on the stack, say)
     |
     v
A ~> B
^    |
|    v
D <- C

     @
     |
     v
A ~> B
^    |
|    v
D <- C

     @
     |
     v
A ~> @
^    |
|    v
D <- C

     @
     |
     v
$ ~> @
^    |
|    v
$ <- $

     @
     |
     v
$ ~> $
^    |
|    v
$ <- $
```

Great!

Note that if `owner` had a pointer to any other node than B, part of the cycle
will be dropped before `owner`.

However weak pointers are fundamentally something that you have to pay for
*even if you're not using them*. Every RcBox will consume an extra pointer of
space on the heap, and the code needs to include checks for weak-pointer
conditions regardless.

Note that we have *never* received complaints about this, to my knowledge. Servo
and rustc happily use Rc without Weak. This isn't so much a "this is tragedy"
argument so much as a "this is philosophically inconsistent". The standard
library generally favours a "pay for what you use" philosophy. There are of
course exceptions for memory-safety (extra checks). However to my knowledge
there are two major exceptions:

* Mutex poisoning
* Weak pointers

Mutex poisoning is a friendly guard against not considering *strong*
exception safety, and I don't want to get into that can of worms. I will say
it actually *was* measured to have negligable impact.

Weak pointers, on the other hand, are simply a feature that bloats users
regardless of whether they use it or not.

The other reason to be suspect of weak pointers is they raise some awkward
questions for the current unstable APIs:

```rust
fn try_unwrap(rc: Rc<T>) -> Result<T, Rc<T>>
fn downgrade(&self) -> Weak<T>
fn weak_count(this: &Rc<T>) -> usize
fn strong_count(this: &Rc<T>) -> usize
fn is_unique(rc: &Rc<T>) -> bool
fn get_mut(rc: &mut Rc<T>) -> Option<&mut T>
fn make_unique(&mut self) -> &mut T
```

In particular *what is uniqueness* when you have weak pointers?

# Detailed design

Deprecate Weak pointers in favour of an external crate and stabilize the
following methods on Rc and Arc:

```rust
fn try_unwrap(rc: Self) -> Result<T, Self>
fn get_mut(rc: &mut Self) -> Option<&mut T>
fn make_unique(rc: &mut Self) -> &mut T
```

Some notes:

We aren't stabilizing `is_unique` and `strong_count`. The former
is almost always expressible as `get_mut().is_some()` in a pinch, and the
latter has questionable value (since uniqueness is the only *really*
interesting state).

make_unique in now a static method to align with our general
policy on avoiding normal methods on smart pointers.

Arc doesn't currently have `try_unwrap` but this is considered an oversight.

# Drawbacks

Weak pointers aren't useless, and the full tools to properly implement Rc and
Arc aren't available on stable Rust. This would effectively slow down the
ability to use weak pointers. However it's possible to limit the functionality
a bit and use a few hacks to get the same basic functionality.

Everyone using Weak is on nightly already anyway.

# Alternatives

Accept the tiny overhead and stabilize Weak. This involves answering the
question of what uniqueness means in the context of Weak pointers.

# Unresolved questions

Nah
