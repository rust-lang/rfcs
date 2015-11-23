- Feature Name: thread_local
- Start Date: 2015-11-23
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

* Add the `Interrupt` marker trait for thread-local objects that can be used
  from signal handlers.
* Relax the requirements for `#[thread_local]` statics from `Sync` to
  `Interrupt`.
* Tighten the requirements for `thread_local!` statics from none to `Interrupt`.

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

Such requirements are not suitable for the rust language since all compiling,
safe code must have defined behavior. Either

1. rust code must never be called from signal handlers and there must not be a
   safe way to create a signal handler in rust; or
2. there must be compiler restrictions so that rust signal handlers can only
   call functions that have in some way be marked safe to be called from signal
   handlers; or
3. all safe rust code must be safe to be called from signal handlers.

This RFC proposes a way to realize the third option.

## The current state of thread local data in rust

### Via the `#[thread_local]` attribute

```rust
#[thread_local]
static DATA: RefCell<u8> = RefCell::new(0);
```

The `#[thread_local]` attribute marks a static as being thread local. This means
that every thread has its own copy of the static, initialized with the original
value. This is highly efficient but requires support by various parts of the
system and is currently feature gated.

Just like ordinary statics, `#[thread_local]` statics require the type to
implement the `Sync` trait. In particular, the code posted above would not
compile as `RefCell` is not `Sync`.

### Via the `thread_local!` macro

```rust
thread_local!(static DATA: RefCell<u8> = RefCell::new(0));
```

This macro creates a thread local variable that can be accessed as follows:

```rust
DATA.with(|data| {
    // use data
})
```

Note that the variable `data` passed to the closure is an immutable reference.
Hence, the only way to mutate thread local data created with the `thread_local!`
macro is via interior mutability (`RefCell`, `Cell`, `Mutex`, etc.)

This is less efficient than the previous variant because every time `with` is
called, it has to be checked whether the variable has already been initialized,
and, if not, the expression on the right hand side has to be executed.

On the other hand, this version is more flexible:

* The right hand side of the definition is not restricted to expressions that
  can be evaluated at compile time.
* The macro does not require the type to be `Sync`. The code posted above does
  compile.

#### Async-signal-unsafety of the `thread_local!` macro

A thread local variable created with the `thread_local!` macro is not safe to be
used from signal handlers if it contains a `RefCell`. Consider the following
code:

```rust
/// Contract: The first and the second value are always the same.
thread_local!(static X: RefCell<(usize,usize)> = RefCell::new((0,0)));

fn main() {
    X.with(|x| {
        let mut x = x.borrow_mut();
        x.0 += 1;
        x.1 += 1;
    });
}
```

With optimization enabled, this translates to the following pseudo-assembly:

```
main:

 1:  if X is not initialized:
 2:      initialize X
 3:  endif
 4:  if x is borrowed:
 5:      panic
 6:  endif
 7:  increase x.0 by 1
 8:  increase x.1 by 1
```

The missing step is after `6`. The compiler does not emit an instruction to mark
`x` as borrowed. With this instruction, it would look like this:

```
 6:  endif
 7:  mark x as borrowed
 8:  increase x.0 by 1
 9:  increase x.1 by 1
10:  mark x as not borrowed
```

The compiler does not emit these instructions because it does not believe that
the difference can be observed. However, consider what happens when the
following signal handler is invoked after operation `7` but before operation `8`
in the original listing:

```rust
extern fn handler(_: i32) {
    X.with(|x| {
        let x = x.borrow();
        println!("{:?}", *x);
    });
}
```

Since `x` was never marked as borrowed, the borrow succeeds and `(1, 0)` is
printed.

This shows that, in its current state, rust code must not access thread local
variables declared with `thread_local!` inside of signal handlers.

# Detailed design
[design]: #detailed-design

## Changes to the language

A new language item and marker trait `Interrupt` is added to the language:

```rust
/// Types that can be safely accessed from signal handlers.
///
/// The precise definition is: a type `T` is `Interrupt` if `&T` is
/// async-signal-safe. In other words, there is no possibility of data
/// inconsistency when `&T` is used inside a signal handler.
///
/// [...]
#[lang = "interrupt"]
pub unsafe trait Interrupt {
    // Empty
}

unsafe impl Interrupt for .. { }
```

The types that are excluded from `Sync` by default are also excluded from
`Interrupt`:

```rust
impl<T> !Interrupt for *const T { }
impl<T> !Interrupt for *mut T { }
impl<T> !Interrupt for UnsafeCell<T> { }
// etc.
```

All types that are `Sync` are also `Interrupt`:

```rust
unsafe impl<T: Sync> Interrupt for T { }
```

Statics marked with `#[thread_local]` will accept exactly those types that
implement the `Interrupt` trait.

## Changes to the standard library

`thread_local!` is changed to accept only `Interrupt` types. This is a breaking
change, however, the author believes that it is easily mitigated for the
following reasons:

1. Thread local storage is only useful with (interior) mutability.
2. Thread local storage created with `thread_local!` only allows interior
   mutability.
3. The types most commonly used for interior mutability are `Cell`, `RefCell`,
   and `Sync` types with interior mutability.

Hence, the author believes that almost all cases of broken code will be caused
by either `Cell` or `RefCell`.

`RefCell` can be fixed easily: Since there already is a locking mechanism, one
only has to ensure that the locking instructions will actually be emitted. Since
this already happens in almost all cases, this does not cause performance
regressions in the common case. Afterwards in can implement the `Interrupt`
trait.

`Cell` is harder to fix as it has no locking mechanism. Users of `Cell` should
switch to `RefCell` in thread local variables.

The `thread_local!` macro must also ensure that it is only initialized once. If
the initialization process is interrupted and re-entered in a signal handler,
the process must be aborted (since you cannot unwind out of a signal handler.)
For this reason it is suggested that a `try_with` function is added that returns
an error instead of aborting the process.

## Example

The following example is adapted from lrs which tries to be async-signal-safe:

```
/// Stores the closures passed to the function below.
#[thread_local]
static AT_EXIT: SingleThreadMutex<AtExit> = /* ... */

/// Adds a closure to be run when the thread exits.
///
/// [argument, f]
/// The closure that will be run.
///
/// [return_value]
/// Returns whether the operation succeeded.
///
/// = Remarks
///
/// This function should not be called from signal handlers but can be called
/// during the execution of a registered function. If this function is called in
/// a signal handler that was invoked during an invocation of this function, the
/// `ResourceBusy` error is returned.
pub fn at_exit<F>(f: F) -> Result
    where F: FnOnce() + 'static,
{
    let at_exit = match AT_EXIT.try_lock() {
        Some(g) => g,
        _ => return Err(error::ResourceBusy),
    };

    at_exit_inner(f, at_exit)
}
```

# Drawbacks
[drawbacks]: #drawbacks

* Adding a marker trait is a significant change.
* This is a breaking change.
* C functions called from rust code do not magically become signal safe.
  However, the change proposed here is at a more fundamental level: In a perfect
  world where no C functions are called, should static variables be safe to use
  in all cases? In a perfect world where no C functions are called, should
  thread local static variables be safe to use in all cases? The answer to both
  questions is yes, yet only the first case is implemented at this point. It is
  already possible to write kernels and even libc implementations in rust that
  don't call any C code. A forward thinking language---such as rust---should not
  let the opportunity to fix a decades-old problem pass just because there is
  lots of legacy code out there. Even if the current standard library depends
  too much on C code to allow safe async signal handling right now, this might
  change in the future.
* `Interrupt` is a long name. The name `Async` comes to mind, but unlike `Sync`,
  `Send`, and `Interrupt`, `Async` is not a verb.

# Alternatives
[alternatives]: #alternatives

* Do nothing

`#[thread_local]` accepts `Sync` and `thread_local!` accepts everything. This
means that rust code will not be async-signal safe and `#[thread_local]`
requires too much.

* Change `#[thread_local]` to accept everything

This makes `#[thread_local]` as unsafe as `thread_local!`.

* Change both to require `Sync`

This breaks too much.

* Change `#[thread_local]` to require `Interrupt`.

This fixes the language only. The standard library stays signal unsafe. This is
an interesting solution because it accepts the reality of the standard library
(depends on lots of signal-unsafe C code and cannot provide a safe signal
handler interface) while allowing code that doesn't depend on C code to use
thread local variables in a safer way.

# Unresolved questions
[unresolved]: #unresolved-questions

None at this point.
