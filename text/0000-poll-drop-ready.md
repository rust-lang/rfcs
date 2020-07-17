- Feature Name: `poll_drop_ready`
- Start Date: 2020-07-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a `poll_drop_ready` method to `Drop`, and change the drop glue in async
contexts to call this before calling `drop`. This will allow users to perform
async operations during destructors when they are called in an async context.

# Motivation
[motivation]: #motivation

Rust encourages an idiom which originated in C++ called "Resource Acquisition
Is Initialization," or "RAII." This name is very unclear, but what it means is
that objects in Rust can be used to represent program state and IO resources,
which clean themselves up when they go out of scope and their destructors run.
As Yehuda Katz wrote in 2014 [Rust means never having to close a
socket.][close-a-socket]

However, there is no way to perform a *nonblocking* operation inside a
destructor today without just blocking the thread the destructor is running on. 
This is because destructors have no way to yield control, allowing the executor
to run other tasks until this operation is ready to continue. This means that
the performance advantages of our nonblocking and concurrent Future constructs
do not apply to destructor clean up code. It would be preferable for these
constructs to be able to use nonblocking IO when they are used in an async
context.

I'll describe a few concrete examples.

## Flushing and buffered writers

It's fairly common to create `Write` types which perform flush, fsync or even
writes in their destructor. For types which are intended to perform
asynchronous writes, its not possible to do this as a part of the destructor
code.

For example, the std `BufWriter` type is designed to take a series of small
writes and perform a smaller number of large writes at once. It possibly
performs this in its destructor, to guarantee that all writes made to it
ultimately go through to the underlying `Write` type. A similar construct for
async code would have to either: a) flush in a blocking manner, b) spawn a 
new task onto some sort of executor to perform the flush, c) not flush at all
in the destructor, opening the user up to missed writes when they forget to
flush before dropping.

The [`BufWriter` type in `async-std`][bufwriter], for example, currently does
not flush in its destructor because it cannot do so asynchronously. With
`poll_drop_ready`, it would be able to perform an asynchronous flush as a part
of its destructor code.

## Types can which close asynchronously

Types which guard file descriptors usually close the file descriptor when they
are dropped. Some interfaces, like io-uring on Linux, allow closing file
descriptors to be performed asynchronously.  This cannot be performed by the
destructor, because it cannot yield control.

## Types which update internal program state in their destructors

It's not only IO that could be made non-blocking. Some types update state that
is internal to the program when they are dropped, using types like mutexes and
channels. If these programs want to use the nonblocking version of these
constructs, so that only this task waits for the channel to have room or the
mutex to be free, this is not currently possible in their destructor. Instead,
they have to block the entire thread.

## Scope guards

One pattern used in Rust is the "scope guard" - a guard which cleans up state
after user code has executed. There are important caveats to implementing this
pattern safely in a public API (which are better covered in discussions of
memory leaks and destructors), but it can be done safely by passing a reference
to the scope guard to a higher order function. In these examples, the scope
guard implements a destructor which performs the clean up. This way, even if the
client code panics and unwinds, the clean up gets performed. That clean up
cannot be asynchronous today, because the destructor has no way to yield
control.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `Drop` trait gains one new method, with a default implementation:

```rust
trait Drop {
    fn drop(&mut self);

    fn poll_drop_ready(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        Poll::Ready(())
    }
}
```

Like `Drop::drop`, it would not be possible to call `poll_drop_ready` on a
type (users should use `mem::poll_drop_ready` discussed below instead).

When a value is dropped inside of an async context (that is, an async function
or block), it's `poll_drop_ready` method will be called repeatedly until it
returns `Poll::Ready(())`. Then, its `drop` method will be called. This way,
users can perform (or prepare to perform) nonblocking operations during
destructors when they are called in an async context.

It's important to note, however, that `poll_drop_ready` may be called even
after it has returned `Poll::Ready(())`. This is different from the `drop`
method, which is generally guaranteed to be called only once. Users
implementing `poll_drop_ready` should take care to ensure that it has "fuse"
semantics - that once it returns `Ready(())`, it continues to return that value
thereafter.

These additional APIs are also added to the `std::mem` module:

```rust
// an empty async function
async fn drop_async<T>(to_drop: T) { }

fn poll_drop_ready<T>(&mut self, cx: &mut Context<'_>) -> Poll<()>
    where T: ?Sized
{
    // implemented through a lang item
}
```

The `drop_async` function is analogous to the `drop` function, and is also
added to the prelude as well. This function drops the value in an async
context, guaranteeing that its `poll_drop_ready` method will be called.

The `poll_drop_ready` function calls this value's "drop ready glue" - it calls
`poll_drop_ready` on this value and also on all of its fields, recursively. The
exact ordering and semantics of this glue are specified in the reference
section.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Guarantees about when `poll_drop_ready` is called

In general, users cannot assume that `poll_drop_ready` will ever be called,
just as they cannot assume that destructors will run at all. However, they can
assume less about `poll_drop_ready` than they can about `drop`, and that should
be noted.

In particular, users cannot safely assume that their values will be dropped in
an async context, regardless of how they are intended to be used. It's this
easy to supress the `poll_drop_ready` call of a value:

```rust
async fn ignore_poll_drop_ready<T>(x: T) {
    // by passing `x` to `drop`, a non-async context, `poll_drop_ready` is
    // never called when dropping `x`.
    drop(x);
}
```

However, we do guarantee that values dropped in an async context *will* have
`poll_drop_ready` called, and we even guarantee the drop order between
variables and between fields of a type. When the user sees a value go out of
scope *in an async context*, they know that `poll_drop_ready` is called. And we
do guarantee that the destructors of all fields of that type will be called.

## Changes to destructor calls in async contexts

In async contexts, we change the generated destructor calls to insert, before
each call, this code:

```rust
while let Poll::Pending = mem::poll_drop_ready(obj, cx) {
    yield Poll::Pending;
}
```

This "prepares" the object to be destroyed by calling its destructor, calling
`poll_drop_ready` on the object and all of its fields. This is called before
any destructors are run on that object.

In non-async contexts, no change is made to destructors. This means that users
writing low level primitives which may want to run async destructor code on
values they are destroying (such as inside the poll method of a
manually-implemented future) will need to call `mem::poll_drop_ready`
themselves, or this code will be skipped.

Therefore, there are two cases in which users will have to take special care to
ensure that async destructors are called:

1. If they would already have to call `drop_in_place` to ensure that normal
   destructors are called (as in data structures like `Vec`).
2. If they are dropping a value that may have an async destructor inside a
   poll method.

## Drop glue in `mem::poll_drop_ready`

`mem::poll_drop_ready` will first call `Drop::poll_drop_ready` on this value,
before recursively calling `mem::poll_drop_ready` on every field of the value.
It will logically "AND" all of the return values, so that it will return
`Poll::Ready` only if all calls return `Poll::Ready`, and otherwise return
`Poll::Pending`. In psuedo-code:

```rust
let mut ready = self.poll_drop_ready(cx);
$(
    ready &= mem::poll_drop_ready(&mut self.fieldN, cx);
)*
ready
```

### The necessity of fused semantics

Calls to `poll_drop_ready` occur in a loop, until every recursive subcall
returns `Ready`. These calls are (by necessity) stateless: we would be required
to add secret state to every struct which has `poll_drop_ready` to enable the
drop glue to work. We used to have similar secret drop flags to support normal
destructors, but have managed to eliminate it in RFC 320. Consistent with that
philosophy, we want the `poll_drop_ready` glue to be stateless as well. What
this means is that every iteration of the loop, we will call the
`poll_drop_ready` function on each field again, until all of them return Ready.

It is necessary, therefore, when implementing `poll_drop_ready`, to prepare for
the possibility that it will be called after it has returned `Ready`. The best
way to write this is to give it "fused" semantics: have a final state it enters
into from which it will always return `Poll::Ready`.

Futures in general don't have fused semantics because the value they return on
Ready cannot necessarily be manufactured more than once. Because
`poll_drop_ready` evaluates to a Poll of unit, which is a zero sized type that
is trivial to produce, it is much more straightforward to implement it with
fused semantics.

### Why `poll_drop_ready` does not use `Pin`

It would have been ideal for `poll_drop_ready` to use `Pin`, just like `poll`
does. However, this would be unsound because of the definition of the Drop
trait. This is a reoccurrence of an inconvenience caused by the definition of
Drop, as it relates to pin projections.

As first context: the definition of Drop is in some sense "wrong." It would be
preferable of `Drop::drop` took self as `Pin<&mut Self>`, just like
`Future::poll` does. Once something is dropped, we know it won't be moved again
outside of the destructor. But because Drop receives an unpinned mutable
reference, we know that it *can* move self, or even a field of self.

Moving *fields* of self is a particular problem because of how it interacts
with pin *projections* - that is, taking a Pin reference to type and
"projecting" to a pinned reference to a field of that type. If you move out of
the fields of a struct in the struct's destructor, pin projecting to that field
would be unsound, because it would have been moved after it was pinned and
before its destructor ran.

This applies equally well to `poll_drop_ready`. If `poll_drop_ready` took self
by `Pin`, in order to generate drop glue it would need to pin project to each
field of the type. The order of operations would be:

1. `poll_drop_ready` runs, pin projecting to each field of the type.
2. The type's destructor runs, potentially moving each field.
3. Each field's destructor runs, after the type's destructor has potentially
moved them.

Because of this sequence, the in projection in `poll_drop_ready` would be
unsound. Therefore, `poll_drop_ready` cannot take self by Pin. Here is a bit of
code demonstrating what I mean:

```rust
// If `field` implements `poll_drop_ready` and `poll_drop_ready` used `Pin`,
// `field` would be witnessed as pinned before calling drop on `Foo`.
struct Foo<T: Default> {
    field: T
}

impl<T: Default> Drop for Foo<T> {
    fn drop(&mut self) {
        // Then, in the destructor for `Foo`, we can move `field` and re-pin it
        // at a different location, violating the guarantees of `Pin`.

        let moved_field = mem::replace(&mut self.field, T::default());
        let pinned: Box<Pin<T>> = Box::pin(moved_field);
    }
}
```

However, its worth noting that a user can locally reason about whether private
fields can be moved or accessed again, based on the APIs exposed by a type,
and, if necessary, reconstruct a pin of those fields. [The pin documentation
contains relevant information.][pin-docs] In other words, if your drop
implementation is implemented correctly, you can construct a `Pin` of self
inside your `poll_drop_ready`.

## Drop order

The order of calls to `poll_drop_ready` can be inferred from the rest of this
document, but it's good to specify them explicitly.

Between variables, calls to `poll_drop_ready` will occur in reverse order of
the variable's introduction, immediately prior to the calls to that variable's
destructor.

Between fields of a type, calls to `poll_drop_ready` will occur in the textual
order of that type's declaration, with the call for the type itself occuring
first. This is similar to the order for calls to drop. *However*, these calls
will occur in a loop, at the level of the type being dropped, until all calls
to `poll_drop_ready` for that value have returned `Ready`, so they will be
"interleaved" and concurrent in practice. The program will *not* wait for each
field to return ready before beginning to process the subsequent field.

# Drawbacks
[drawbacks]: #drawbacks

Like all language extensions which increase the expressiveness of the language,
this feature will increase the complexity of Rust. In particular, this feature
sits squarely at the intersection of async code and destructors, both areas of
the language which involved nuanced guarantees about memory and correctness.
Therefore, the biggest drawback of adding this feature is that it increases the
complexity of the language.

It should also be noted that it introduces silent "yield points" into async
contexts, whereas previously all yield points were visible awaits. This is just
an extension of the normal behavior of destructors, which already can block,
panic, take locks, park the thread, mutate global state, and so on (as they are
intended to do).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There is a big trade off in this space between guaranteeing the async
destructor code will be called in as many places as possible and making it as
ergonomic as possible to implement async destructors. This RFC leans heavily
toward designing `poll_drop_ready` to be called in as many instances as
possible, even if it makes `poll_drop_ready` less straightforward to implement.

We believe this decision is well-motivated for several reasons. First, it
pushes complexity to the implementor of the async destructor, rather than
placing more burdens on users of types with async destructors to ensure that
the async destructor gets called. We believe that it is appropriate that the
libraries which want to use this feature manage the complexity of using it as
much as possible, and end users get the benefits with as little effort as
possible.

Second, the rules we have arrived for when `poll_drop_ready` gets called at are
a fairly straightforward extension to the current destructor rules:
`poll_drop_ready` is guaranteed to be called whenever a value is dropped in an
async context. Creating more complex exceptions to that rule would not only put
the burden on end users, but also make the feature more complex for those same
end users to understand when they need to manually ensure `poll_drop_ready` is
called. Importantly, we do guarantee that `poll_drop_ready` is called on all
fields of a type, avoiding the requirement for every type which could contain a
field with an async destructor having to implement a "forwarding async drop" to
ensure the field's destructor is called.

That said, there are alternatives along this continuum that make it nicer to
implement an async destructor while guaranteeing that it will be called in less
circumstances, and we should document them.

## Have no async drop glue

The first step would be to have no async drop glue: we would only drop values
when they themselves are dropped in an async context, not when types which have
a field of that value are dropped in an async context.

This would enable us to make two modifications to the design:
* `poll_drop_ready` could take self by `Pin`
* `poll_drop_ready` would not need to have fused semantics

Because there's automatic drop glue generated for fields, we would not need to
automatically pin project to them to have them take self by pin. And because
there's no drop glue that needs to be stateless, we would not need fused
semantics.

However, this would make it the responsibility of the author of every type
which has a field which implements `poll_drop_ready` to implement the drop glue
as a part of their own `poll_drop_ready` implementation. And if we eliminate
fused semantics and make them take self by pin, that drop glue also cannot be
as simple to implement as the drop glue this RFC proposes we auto generate. We
would essentially place all of the burden on everyone who wants to guarantee
the `poll_drop_ready` of their field gets called.

Note that "any type which has a field which implements `poll_drop_ready` means,
in practice, *any generic type*, because any generic parameter could possibly
implement `poll_drop_ready`. This would be incredibly burdensome across the
ecosystem.

## Have no virtual async drop, more weirdness around auto traits

The ideal and most immediately obvious API for an async destructor is something
like this:

```rust
trait AsyncDrop {
    async fn drop(&mut self);
}
```

However, this introduces several problems because it would allow the body of
the async fn to introduce arbitrary additional state not contained in the type
itself. This would cause the same problems that non-fused semantics would cause
for drop glue, but it would also cause even more problems.

The first is that we would not be able to async drop a trait object. When a
trait object is dropped, we dynamically dispatch the call to the destructor of
the concrete type that backs that object, through the vtable of the trait
object. However, this definition of async drop would require some place to put
the state of the future the async call returns, and the layout of each future
for each concrete type would be different and unknown. It would be functionally
equivalent to having a trait object with an unknown associated type, which is
not allowed for exactly this reason. It would not be possible to support async
destructors on trait objects.

It would also make the problem of futures created with async contexts not
implementing `Send` or `Sync` worse. If the state of the future returned by the
async drop was not `Send` or `Sync`, neither would any async context in which
that type is dropped be `Send` or `Sync`.

In other words, even though the user cannot call the async drop method of a
type, using an async fn for this purpose would introduce many of the
limitations of async fn in traits, and would not be ideal for the use case of
destructors.

## Putting `poll_drop_ready` in a separate trait

Possibly, `poll_drop_ready` should not be a provided method of Drop, but should
be in a separate trait. This would increase the complexity of the API, but make
it possible to bound by that trait. In general, bounding by Drop is usually an
antipattern that doesn't do what the user wants and there's not really any use
in creating a separate trait for this method either.

# Prior art
[prior-art]: #prior-art

No other language seems to have a way to call asynchronous code from the
destructor without spawning a new task. There is one major factor that most of
those languages have in common: they are garbage collected, and so their
equivalent to destructors (usually called finalizers) cannot be easily
guaranteed to run at any particular time. For that reason, they are not used
for clean up of external resources in the "RAII" sense that they are used in
Rust and C++.

C++ has a coroutine proposal similar to async/await. It has not yet been
extended to allow for something like async destructors.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

## Clippy lint

One footgun that users may encounter is calling `mem::drop` inside of an async
context. Unless they intended to supress the `poll_drop_ready` of this value,
or know that this type will never have a meaningful `poll_drop_ready`, they
should call `mem::drop_async`.

Clippy could add an opinionated lint on calls to `mem::drop` in an async
context, encouraging users to call `mem::drop_async` instead. This lint would
belong in clippy and not the compiler, because it has a high possibility of
false positives: supressing the `poll_drop_ready` calls and yield point is a
valid and correct use of the `drop` call if it's done intentionally.

## Optimizing out `poll_drop_ready` calls

We should optimize away the states in futures yield for trivial
`poll_drop_ready` calls that will never actually yield. It may be that this
falls out of existing optimizations, but if not we should explore the options
to perform this optimization effectively.

## Supporting the ecosystem in calling `poll_drop_ready`

Generic futures combinators should be written to ensure that `poll_drop_ready`
is called on types they drop which may have implemented the `poll_drop_ready`
method. Toward this end, we should endeavor to provide good documentation
informing the authors of those combinators of what they should do and how they
can do it, open issues on major libraries which implement combinators, and
ensure that libraries under the control of the Rust project (e.g. futures-rs)
guarantee calls to `poll_drop_ready`.

[bufwriter]: https://docs.rs/async-std/1.5.0/async_std/io/struct.BufWriter.html
[close-a-socket]: https://blog.skylight.io/rust-means-never-having-to-close-a-socket/
[pin-docs]: https://doc.rust-lang.org/std/pin/index.html#drop-implementation
