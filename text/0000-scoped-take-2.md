- Feature Name: scoped
- Start Date: 2015-04-16
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC proposes an alternative to the `thread::scoped` API that does not rely
on RAII, and is therefore memory safe even if destructors can be leaked.

*This RFC was inspired by ideas from @nikomatsakis and @arielb1.*

# Motivation

The `thread::scoped` API allows parent threads to launch child threads that can
have references to *stack data* owned by the parent. This style of API makes
fork-join programming easy and efficient. It is also safe, *as long as the
parent's stack frame outlives the children threads*.

In the initial version of the API, safety was "guaranteed" through an RAII-style
guard which, upon destruction, would join (wait for) any still-living children
threads. This join guard is connected to the lifetime of any borrows by children
threads, thereby (we thought) preventing the related parent stack frames from
being popped until the joins were complete.

Unfortunately,
[it is possible to avoid the guard destructor in safe Rust](https://github.com/rust-lang/rust/issues/24292). Rust
does not guarantee that the destructor for a value that escapes will be run.

Note that this does *not* mean that RAII should be avoided in general. **RAII is
still a safe, idiomatic, and ergonomic style of programming for Rust.** It just
means that you cannot write `unsafe` code whose safety relies on RAII to run
destructors.

It may be possible to finesse this issue by locating every possible source of
destructor leakage (such as `Rc` cycles) and tying it to some kind of "`Leak`"
marker trait, which would be implemented by default for all Rust types (and
basically means "safe to leak").  Ideas along these lines have been discussed
extensively [elsewhere](https://github.com/rust-lang/rfcs/pull/1066), but they
represent longer-term language changes that may or may not work out.

This RFC, by contrast, proposes a general API that works in today's Rust, and
can:

* Guarantee execution of a piece of code prior to a scope being exited, even in
  the presence of unwinding, `Rc` cycles, and so on.

* Thereby be used to recover a safe `thread::scoped` API.

While the API proposed here is slightly less ergonomic than the RAII-style
`scoped` API, **it has the benefit of much more clearly marking the point at
which children threads will be joined, which was left more implicit in the old
API.**

# Detailed design

The proposal has two pieces: a general mechanism for "deferred computation",
and a new `thread::scoped` API that takes advantage of it.

## Deferred computation

Here's a very simple API for deferring computation:

```
// in std:

mod thread {
    pub struct Scope<'a> { ... }

    pub fn scope<'a, F, R>(f: F) -> R where F: FnOnce(&Scope<'a>) -> R;

    impl<'a> Scope<'a> {
        pub fn defer<F>(&self, f: F) where F: FnOnce() + 'a;
    }
}
```

(This is put in the `std::thread` module because (1) scopes are per-thread
concepts (2) it fits nicely with other functionality like `panicking` and
`catch_panic`.)

You call `scope` to introduce a new `Scope` value, which is passed into a
closure that is immediately invoked.

The closure can use the `defer` method to register callbacks to invoke upon
*any* exit from the callback, including unwinding. Unlike RAII guards, there is
no way to leak these callbacks, and the implementation shown below works around
[known cases](https://github.com/rust-lang/rust/issues/14875) of destructor
leakage.

This is a generally useful mechanism that can avoid the need to create custom
RAII guards for situations where you might use `try`/`finally` in other
languages. But it is also just the support needed for scoped threads.

## Scoped threads

To recover scoped threads, we extend the `Scope` type with a method for spawning
threads:

```rust
impl<'a> Scope<'a> {
    pub fn spawn<F, T>(&self, f: F) -> thread::JoinHandle<T> where
        F: FnOnce() -> T + Send + 'a,
        T: Send + 'a;
}
```

Like the original `thread::scoped` API, this allows for the child thread's
closure to borrow data (with lifetime `'body`) from the parent thread. These
borrows are bounded by the lifetime of the `Scope` value, and the implementation
uses the `defer` method to add a callback that will join on (wait for completion
of) the child thread on exit from the scope -- thus restoring memory safety.

Note that, while previously one might return `JoinGuard`s outward to expand the
scope of joining, the pattern here is reversed: you call `scope` at the
outer-most scope, and then pass a reference to the `Scoped` value inward.

Putting it all together, here's an example from TRPL, and a version adjust to the new API:

```rust
// using the thread::scoped API
fn old_trpl_example() {
    let data = Mutex::new(vec![1u32, 2, 3]);

    let guards: Vec<_> = (0..2).iter().map(|_| {
        thread::scoped(|| {
            let mut data = data.lock().unwrap();
            data[i] += 1;
        })
    }).collect();

    // threads implicitly joined here, when `guards` goes out of scope and drops
    // its contents
}
```

```rust
// using the proposed thread::scope API
fn new_trpl_example() {
    let data = Mutex::new(vec![1u32, 2, 3]);

    thread::scope(|s| {
        for i in 0..2 {
            s.spawn(|| {
                let mut data = data.lock().unwrap();
                data[i] += 1;
            });
        }
    })
}
```

In the original version of the example, the join guards from the scoped threads
were explicitly collected into a vector (to ensure that the joins did not happen
too early).

With the new version, by contrast, the scope is more clearly marked by an indent,
all *all* joins within the scope automatically happen at the end of the block,
making scoped threads feel more like a first-class control-flow construct.  In
this RFC author's opinion, this actually winds up *clarifying* the semantics of
scoped threads, and so may be a better API than the original `thread::scoped`.

On a separate note, in practice one will usually not want to spawn full-fledged
threads when doing data-parallel, fork-join style computations; instead you want
to use a thread pool, work stealing, and higher-level combinators. One nice
aspect of `Scope` is that crates providing such parallelism frameworks can
easily hook into the API with their own means of spawning lightweight tasks;
this should also facilitate a [decoupling of thread pools from the lifetimes
bounding the tasks they are running](https://github.com/rust-lang/threadpool/issues/7).

## Implementation

Here's a sketch of the implementation, inspired in part by
[@arielb1's ideas](https://gist.github.com/arielb1/5eb299a87546ce8829b3):

```rust
pub struct Scope<'a> {
    dtors: RefCell<Option<DtorChain<'a>>>
}

struct DtorChain<'a> {
    dtor: Box<FnBox() + 'a>,
    next: Option<Box<DtorChain<'a>>>
}

pub fn scope<'a, F, R>(f: F) -> R where F: FnOnce(&Scope<'a>) -> R {
    let mut scope = Scope { dtors: RefCell::new(None) };
    let ret = f(&scope);
    scope.drop_all();
    ret
}

impl<'a> Scope<'a> {
    // This method is carefully written in a transactional style, so
    // that it can be called directly and, if any dtor panics, can be
    // resumed in the unwinding this causes. By initially running the
    // method outside of any destructor, we avoid any leakage problems
    // due to #14875.
    fn drop_all(&mut self) {
        loop {
            // use a separate scope to ensure that the RefCell borrow
            // is relinquished before running `dtor`
            let dtor = {
                let mut dtors = self.dtors.borrow_mut();
                if let Some(mut node) = dtors.take() {
                    *dtors = node.next.take().map(|b| *b);
                    node.dtor
                } else {
                    return
                }
            };
            dtor()
        }
    }

    pub fn defer<F>(&self, f: F) where F: FnOnce() + 'a {
        let mut dtors = self.dtors.borrow_mut();
        *dtors = Some(DtorChain {
            dtor: Box::new(f),
            next: dtors.take().map(Box::new)
        });
    }
}

impl<'a> Drop for Scope<'a> {
    fn drop(&mut self) {
        self.drop_all()
    }
}
```

This implementation does a few interesting things:

* It avoids any allocation in the case of a single deferred computation.

* It works around issue 14875 in a somewhat subtle way: the `drop_all` method is
  called *both* normally within `scope`, and in the `Scope` destructor. The
  method is also coded transactionally. This means that the first panic (if any)
  in a deferred computation triggers the drop for `Scoped` (without any leakage,
  avoiding #14875), and any remaining panics wind up aborting.

Note that this is just a sketch: the use of a linked list here, and `RefCell`
internally, could both be switched for something more interesting.

# Drawbacks

The main drawback of this approach is that it is arguably less ergonomic (and
less pretty) than the original `thread::scoped` API. On the flip side, as the
RFC argues, this API makes the control-flow/synchronization of the joins much
clearer, resolving a tension in the design of `thread::scoped` (which `must_use`
only somewhat mitigated).

# Alternatives

The main alternative would be to make the original `thread::scoped` API safe by
changing some other aspect of the language. Ideas along these lines are being
[debated elsewhere](https://github.com/rust-lang/rfcs/pull/1066).

# Unresolved questions

Is there any reason that `Scope` should be `Send` or `Sync`?
