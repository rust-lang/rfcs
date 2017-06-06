- Start Date: 14-09-2014
- RFC PR: 
- Rust Issue: 

# Summary

Allow the compiler to drop objects that are no longer used before the end of
their scope. Rust's type system ensures that objects aren't dropped too soon.
The programmer can explicitly specify an object's lifetime if necessary.
Removes the need for drop flags.

# Motivation

In C++, it is the programmer's responsibility to ensure an object isn't used
after it is destroyed. To facilitate this, it provides specific rules for when
objects get destroyed (at the end of scope, in reverse order of construction).
This is very important because the compiler doesn't know which objects depend
on which others.

Rust currently follows this practice of dropping objects at the end of scope.
However, Rust's type system is much more powerful than that of C++, allowing
lifetime dependencies between objects to be specified and enforced at compile
time. The vast majority of the time, it doesn't matter where inside its scope
an object is dropped as long as all lifetime dependencies are met. This even
extends to concurrency types such as `Mutex`, since the protected object must
be accessed through the protecting `Mutex`.

By allowing types to be dropped as soon as they are no longer used, we would
further encourage programmers to properly utilize the type system to specify
lifetime dependencies instead of implicitly relying on the current drop
semantics. This would result in safer code, since the compiler would be able to
catch accidental violations of the lifetime dependencies that may be introduced
in the future.

For the rare case that the programmer wants to control an object's lifetime
independently of its dependencies with other objects (e.g., if dropping an
object is time consuming and shouldn't happen in the middle of a
timing-sensitive operation), the programmer would be required to specify the
lifetime explicitly. This makes it clear that the specific lifetime of the
object is important and prevents accidental earlier moves and drops.

It is currently possible for one code path to move an object an another not to.
Currently, Rust handles this case by adding a flag to each object implementing
`Drop` indicating whether or not it has already been dropped. Because an object
can't be used after it has conditionally been moved, allowing eager drops would
enable the compiler to statically ensure that any objects moved or dropped in
one branch are also moved or dropped in the other, eliminating the need for
drop flags. See [RFC PR 210](https://github.com/rust-lang/rfcs/pull/210) for a
discussion of why the current situation is far from ideal.

Allowing eager drops could potentially even reduce memory usage. Because the
stack space of objects that are no longer needed could be reused for newly
created objects, the total stack size required by a function may be able to be
reduced. Additionally, any heap memory controlled by an object could also
potentially be freed sooner.

Finally, there has been some discussion about shortening the duration of
borrows from their current lexical basis in order to allow greater borrowing
flexibility. Allowing eager drops would generalize this idea to objects instead
of restricting it to references. Thus, the same flexibility could be obtained
when using objects that hold references.

# Detailed design

The compiler may drop an object an any time as long as the following conditions
are met:
- There are no future uses of the object
- The object is not currently borrowed
- There is no other existing object which the object in question must outlive

Additionally, an object must be dropped when leaving its scope through any
means.

To explicitly control an object's lifetime, the programmer would call drop at
the point where they want the objects lifetime to end. Since calling drop
counts as a use of the object, the object would be guaranteed to be freed at
exactly that point. Additionally, any attempt to move or partially move from
the object (even conditionally) would be an error, catching any accidental
moves. (In the very rare event that the programmer wants to move or drop an
object in one case and explicitly extend its lifetime in another, they would be
forced to use an `Option`.)

# Drawbacks

- Might be surprising to those coming from C++

- Existing code (especially unsafe code) must be audited to ensure it is not
  implicitly relying on the current drop semantics

- Harder to tell exactly when an object will be dropped

  This shouldn't be an issue in practice. Most of the time one doesn't care as
  long as lifetime dependencies are satisfied, and one can drop explicitly when
  one does care.

- Code specifically examining and manipulating drop flags would have to be
  changed

# Alternatives

Several alternatives have been suggested to specifically address the current
drop flags:

- Unbalanced drops are an error

  This would require that all objects that are moved or dropped in one branch
  be moved or dropped in every other branch. This was found to be too painful.

- Only allow the compiler to drop objects before the end of scope in the case
  of unbalanced conditional drops.

  This is the gist of [RFC PR 210](https://github.com/rust-lang/rfcs/pull/210).
  This approach attempts to be more conservative, changing the current
  semantics the minimum amount necessary to avoid drop flags. Unfortunately,
  this at the expense of consistency. This has many of the same drawbacks as
  more general eager drops, but misses out on many of the advantages outlined
  above. It still allows one to implicitly rely on drops occurring at the end
  of the scope, while at the same time introducing a corner case by which an
  object can be implicitly dropped early, increasing the possibility of a
  mistake. The RFC attempts to mitigate this danger through a system of traits
  and lints, but this is complex and ties the warnings to specific types, which
  is not ideal (see below).

- Stack based dynamic drop flags

  The biggest problem with the current implementation of drop flags is that it
  adds a drop flag to every single object implementing `Drop`, regardless of
  whether that object is ever conditionally moved or dropped, or even could be.
  Instead of doing this, it would be possible to keep drop flags on the stack,
  and only store and check them for objects that are actually conditionally
  moved. This would allow us to maintain the current semantics while
  substantially reducing the overhead.

Additionally, there has been some discussion about determining when to drop an
object based on its type, with objects of some types having eager drop
semantics and objects of other types having scope-based drop semantics.
Unfortunately, this doesn't really work because the types about whose lifetimes
the programmer cares is highly dependent on context. Furthermore, it would make
object lifetimes inconsistent and more complex.

As an example, if one is programming a timing-sensitive, real-time routine,
taking the time to free a large tree in the middle may be completely
unacceptable. On the other hand, if one is doing a long calculation, performing
a drop that performs a large write to disk part way through the calculation
instead of at the end may be perfectly acceptable.

Because of this, it makes sense for drop semantics of an object to be
controlled by the user of the type, not the creator. In the future, if it turns
out there are some types that legitimately do not make sense to use without an
explicit lifetime, we could add a trait or attribute that triggers a warning if
the type is used without one.

# Unresolved questions

- Should objects to always be dropped as early as possible?

  Currently, this RFC leaves it up to the compiler to determine the optimal
  time to drop an object within the relevant constraints. Would it be useful to
  explicitly specify that objects are dropped as soon as they are no longer
  used? Are there any optimizations this would hinder?

- Should this cause the compiler to accept code that wouldn't work today?

  The programmer could potentially take advantage of early drops by, e.g.,
  moving an object that was being by an object eligible for an eager drop (this
  would force the compiler to perform the drop before the move). Is this
  desirable?

- Should there be a `scoped` keyword, attribute, or similar mechanism?

  This RFC avoids adding any additional syntax, using the existing drop
  function to explicitly specify an objects lifetime when needed. Some have
  suggested that it would be nice to add a `scoped` keyword that can be used in
  a `let` statement to specify that a given variable should be dropped at the
  end of its scope. This would have have roughly the same effect as calling
  `drop` at the end of the scope, including disallowing full or partial
  conditional moves. Would this be helpful?

- How would this interact with fail?

  Under this proposal, it's easy for the compiler to statically determine what
  needs to be dropped during normal control flow (e.g., it can determine
  exactly what objects are alive when a give return statement is reached).
  Would the same be true in the case of stack unwinding, or would task failure
  complicate the implementation of this proposal?
