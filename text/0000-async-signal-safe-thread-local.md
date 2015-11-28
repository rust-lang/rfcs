- Feature Name: thread_local
- Start Date: 2015-11-23
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

* Add the `Interrupt` trait for thread-local objects that can be used from
  signal handlers.
* Relax the requirements for `#[thread_local]` statics from `Sync` to
  `Interrupt`.

# Motivation
[motivation]: #motivation

## Asynchronous signal handling in C

In the rationale for C99, section 5.2.3, the following is said about signal
handlers:

>The C89 Committee concluded that about the only thing a strictly conforming
>program can do in a signal handler is to assign a value to a volatile static
>variable which can be written uninterruptedly and promptly return. It is
>further guaranteed that a signal handler will not corrupt the automatic storage
>of an instantiation of any executing function, even if that function is called
>within the signal handler.  No such guarantees can be extended to library
>functions [...] since the library functions may be arbitrarily interrelated and
>since some of them have profound effect on the environment. 

Hence, a pure C99 program must follow very strict rules in order to avoid
undefined behavior. The POSIX standards relax these requirements by specifying a
list of functions that can safely be called from signal handlers.

Thread local data is often part of the problem. A function might temporarily put
such data in an inconsistent state. If a signal handler interrupts such a
function, it might observe the inconsistent state which might cause the behavior
of the signal handler to be undefined.

## Thread local data in the rust language

The rust language currently supports thread local data via the `#[thread_local]`
attribute that can be applied to global variables. The attribute does, however,
not change that those global variables have to implement the `Sync` trait. Since
one cannot distinguish between types that must be safe to access from multiple
threads and types that must only be safe to access from signal handlers, thread
local data is forced to use expensive atomic locking operations.

One can, of course, implement the `Sync` trait without using said locking
mechanisms. But then one has to hope that the type is never accidentally used
in non-thread-local global variables.

For this reason, another trait—`Interrupt`—should be added that allows
one to distinguish between `Sync` types that are thread-safe, `Interrupt` types
that are async-signal-safe, and types which have neither property.

# Detailed design
[design]: #detailed-design

A new language item and trait `Interrupt` is added to the language:

```rust
#[lang = "interrupt"]
pub unsafe trait Interrupt { }
```

A type can be the type of an immutable `#[thread_local]` global variable if and
only if it implements this trait.

# Drawbacks
[drawbacks]: #drawbacks

* `Interrupt` is a long name. The name `Async` comes to mind, but unlike `Sync`,
  `Send`, and `Interrupt`, `Async` is not a verb.

# Alternatives
[alternatives]: #alternatives

Not doing this.

# Unresolved questions
[unresolved]: #unresolved-questions

None at this point.
