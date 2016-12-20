- Feature Name: coroutines
- Start Date: 2016-10-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add language-level support for stackless coroutines (also known as semicoroutines or [generators](https://en.wikipedia.org/wiki/Generator_(computer_programming)))

# Motivation

> Coroutines are computer program components that generalize subroutines for nonpreemptive multitasking, 
> by allowing multiple entry points for suspending and resuming execution at certain locations.  
> \- Wikipedia

The ability to suspend execution of a routine and resume it at a later time comes handy in a number of circumstances:
- iteration over complex data structures 
- asynchronous workflows (suspend execution while waiting for asynchronous operation complete)
- inversion of control (e.g. incremental parsing using recursive descent) 

At present, when faced with one of the above tasks, Rust developers would most likely end up creating a state machine 
by hand ([an example from tokio](https://github.com/tokio-rs/tokio-core/blob/623ce443d89cd9ffa2c1adae8d2eb75538802d01/src/io/copy.rs)).
Iterator/async combinators do provide assistance in simpler cases, however flows with complex conditions and loops 
are usually not amenable to this approach. Coroutines would be of great help here.

Finally, stackless coroutines have been implemented and proved useful in other languages: Python, C#, F#, JavaScript 6, and soon, C++.

See also: [Motivating Examples](#appendix-motivating-examples).

# Detailed design

The subtype of coroutines being proposed in this RFC is technically known as *stackless coroutines* (or *semicoroutines*).
Stackless coroutines are different from [stackful coroutines](#stackful-coroutines) in that only the top level routine may be 
suspended.  By restricting suspensions to the top level, the amount of state associated with each instance of a 
coroutine becomes bounded and may be stored much more compactly than traditional stacks.  
(For the remainder of this document, we'll mostly refer to them simply as 'coroutines'.)

A popular implementation (which we are also going to follow) is to transform coroutine control flow into an explicit state 
machine, with state snapshot stored in a structure, whose pointer is passed to the coroutine code each time it is
resumed (In this respect coroutines are remarkably similar to regular closures with their "closure environments".  
In fact, to an outside observer, a coroutine is not distinguishable from a closure that happens to return a `CoResult`).

### Syntax
Coroutines use the same syntax as regular closures with an addition of a "yield" expression:
```bnf
yield_expr := yield <expr>;
```

Here's an example to get us going:
```rust
let coro1 = || {
    for i in 0..10 {
        yield i;
    }
};
```
The return type of a coroutine is always ```CoResult<Y,R>```, which is a new lang item
defined as follows:
```rust
#[lang="coresult"]
enum CoResult<Y,R> {
    Yield(Y),
    Return(R)
}
```
The two variants of this enum correspond to **yield** and **return** statements.  Callers may use this 
to determine when a coroutine has run to completion:

```rust
while let Yield(x) = coro1() {
    print!("{} ", x); // prints "0 1 2 3 4 5 6 7 8 9 " 
}
```

Coroutines may also have parameters; in this case the first invocation of the coroutine binds passed-in
parameters to the declared arguments, here - a1, a2 and a3.  Parameters passed on subsequent invocations 
are returned as a tuples from **yield** expressions:
```rust
let coro2 = |a1, a2, a3| {
    for i in 0..10 {
        ...
        let (b1,b2,b3) = yield y1;
        ...
        let (c1,c2,c3) = yield y2;
        ...
    }
    ...
    return result;
};
```
For an alternative way of dealing with coroutine parameters see [this](#implicitly-binding-coroutine-arguments).

To recap:
- No coroutine code is executed when it is created (other than initializing the closure environment).
- The first time a coroutine is invoked, execution starts at the top, and the passed in parameters are 
  assigned to coroutine arguments (`a1`, `a2` and `a3` in the `coro2` example). 
- When execution reaches the first yield point, control is returned to the caller, 
  returning the argument of the **yield** expression wrapped in `CoResult::Yield`.
- During subsequent invocations, execution resumes immediately after the last executed yield point. 
  The value of the **yield** expression will be a tuple of the parameters provided by the caller.
- When execution reaches a **return** statement, or falls off the end of the coroutine body, 
  the coroutine returns for the last time, the return value being wrapped in `CoResult::Return`.
- Further attempts to invoke that coroutine shall result in panic.

### Typing
The types of coroutine signature are subject to normal type inference with a few additional constraints:
- All return'ed values must be of the same type `R` (same as for regular functions/closures).
- All yield'ed values must be of the same type `Y`.
- The tuple of coroutine arguments and the return types of all **yield** expressions must be of the same type `A`. 
  In other words, `typeof (a1,a2,a3)` == `typeof (b1,b2,b3)` == `typeof (c1,c2,c3)`.

### Fn* traits
Coroutines shall implement the `FnMut` trait:
- they cannot just implement `FnOnce`, because then the environment would get destroyed after the first yield,
- they cannot implement `Fn`, because at the very least they need to modify the field which keeps
  track of the current state.

### Hoisting of locals
Local variables and temporaries whose lifetime straddles any yield point must be preserved while 
the coroutine is suspended, and so they are hoisted into the coroutine environment.
Note that this only moves their storage location, the lifetimes stay intact.
In the simplest case, each hoisted variable gets its own unique storage space.   A further optimization 
would be to overlay storage of variables which are not live simultaneously. 

### No borrows across yield points
Consider this code: 
```rust
    let a = vec![0; 100];
    let b = a.iter();
    yield c;
    for x in b {
        ...
```
Since both `a` and `b` are live across a yield point, they will be hoisted into the coroutine environment.
Unfortunately, this means that the environment struct would store a reference to another part of self.
This cannot be allowed because if the coroutine environment gets moved while it is suspended, 
the internal pointer to `a` inside `b` would become invalid.  Thus, the compiler must emit an error.  
The above does not prevent usage of references between yield points or having references to external objects. 

### Cleanup
When a regular closure goes out of scope, all of the closed-over variables implementing `Drop` are `drop()`'ped.  

In coroutines, liveness of locals variables hoisted into the closure depends on the current state.
If a coroutine runs to completion, all is well, because its hoisted locals will have been disposed of 
in the course of normal execution.  However when a coroutine closure gets destroyed before reaching a return point, 
some extra clean-up will be required:

```rust
impl Drop for CoroClosure1234 {
    fn drop(&mut self) {
        match (self.state) {
            1 => { /* clean-up locals alive at yield point 1 */ },
            2 => { /* clean-up locals alive at yield point 2 */ }
            ...
        }
    }
}

```

### Translation
- Most rustc passes stay the same as for regular closures.
- Type inference and checking passes are modified to take into account the new rules concerning types of 
  arguments and return values.
- Borrow checking must ensure that no borrows are live across yield points.
- Code generation is modified as follows:
    - A 'state' variable is added into the coroutine environment.
    - Local variables whose lifetime straddles any yield point are hoisted into the coroutine environment.  
    - A "master switch" is added at the top of the function to transfer control to the correct location 
      according to the current state.
    - **yield** expressions are translated as 
        ```rust
        self.state = <N>; 
        return Yield(expr);
        ```
    - **return** statements and the tail expression are translated as 
        ```rust
        self.state = -1; 
        return Return(<expr>);
        ```

Putting all this together, the `coro2` example above would be translated into something like this:
```rust
struct CoroClosure1234 {
    state : int;
    i : int;
    // closed-over variables of the containing function (upvars) also go here
}

impl FnMut<(A1, A2, A3)> for CoroClosure1234 {
    type Output = CoResult<(typeof y1, y2), (typeof result)>;

    fn call_mut(&mut self, a1:A1, a2:A2, a3: A3) -> Self::Output {
        // The body of a coroutine is not expressible in plain Rust,
        // so I am using a MIR-like notation here.
        entry: {
            switchInt (self.state) -> [0:state_0, 1:state_1, 2:state_2, otherwise: invalid];
        } 

        state_0: {
            ...
            self.i = 0;
        }

        bb1: {
            if (self.i < 10) -> [true: bb2:, false: end];
        }

        bb2: {
            self.state = 1;
            return Yield(y1);
        }

        state_1: {
            let (b1,b2,b3) = (a1, a2, a3);
            ...
            self.state = 2;
            return Yield(y2);
        }

        state_2: {
            let (c1,c2,c3) = (a1, a2, a3);
            ...
            self.i += 1;
            goto -> bb1;
        }

        ...
        end: {
            self.state = -1;
            return Return(result);
        }

        invalid: {
            ...
            std::rt::begin_panic(const "invalid state!")
        }
    }
}
```
And on the caller side:
```rust
let coroutine = CoroClosure1234 { state: 0, i = mem::uninitialized(), ... };
while let Yield(result) = coroutine.call_mut(a1, a2, a3) {
    // ... process result
}
```

# How We Teach This

Coroutines are a pretty well established concept in Computer Science.  This particular brand of coroutines 
is likely to be known to new Rust users from other languages.

There will need to ba a new chapter in _The Rust Programming Language_, which should briefly introduce the 
concept and give examples of usage (perhaps the very same as can be found in [appendix](#appendix-motivating-examples)).


# Drawbacks

One drawback of asynchronous code implemented in this style is that it tends to be infectious: 
once there is a single async leaf function, the rest of the code between this function and the root of the 
dispatch loop of the application also needs to be async.  In comparison, [stackful coroutines](#stackful-coroutines) do not suffer 
from this issue; they are transparent to intermediate layers of code. (Though they come with their own problems - see below).  

# Alternatives

## Alternatives to stackless coroutines

### Stackful coroutines

Stackful coroutines are amenable to library implementation and, in the first approximation, do not require language-level support. 
Indeed, [crates.io](https://crates.io/search?q=coroutine) already contains at least half a dozen of such crates.
They also come with some drawbacks:
- Each needs a  stack of at least a few kilobytes in size, which makes them much costlier than needed (our example above
  would have needed only a few dozen bytes for coroutine state). 
- Yielding and resuming requires a register context swap, which is relatively slow.
- Switching stacks is fragile: operating systems often make the assumption that they are the only ones managing 
  thread's register context. 

### Source transformations

There are precedents in other languages of implementing similar functionality via purely source transformations. 
One such example is [F#'s "Computation Expressions"](http://msdn.microsoft.com/en-us/library/dd233182.aspx).  

The exact F# approach is unlikely to work in Rust because of Rust's strong ownership semantics.  F# workflows transform 
control flow constructs into a tree of nested closures, which share variables with the parent closure.  In Rust such sharing 
is prohibited when variables are mutable.

Another plausible approach is a source-to-source transformation into an state machine, where states are represented 
as variants of an enum, each containing hoisted variables that are live during transition to that state. 
An example of such approach is [Stateful](https://github.com/erickt/stateful) 
([blog post](http://erickt.github.io/blog/2016/01/27/stateful-in-progress-generators/)).  
The difficulty here is in computing liveless and types of local variables.  The former is needed to decide 
which variables need to be hoisted, the latter - to declare their types in the "State" enum.
It seems that an implementation would either need to replicate may Rust compiler passes (name resolution,
type inference, possibly borrow checking), or find some clever way to leverage the compiler to compute this 
information.  In the end, it is probably much easier to implement such transformation as a MIR transform, 
which is what this RFC is about.

## Alternatives designs

### Implicitly binding coroutine arguments
Under the current design, the values of parameters with which a coroutine is resumed, are returned as a tuple
from the **yield** expression.
This looks a bit awkward, especially for single-argument coroutines, where it would be cleaner to return the argument 
itself, instead of a 1-tuple wrapping it:
```rust
|a1| {
    let (b1,) = yield x;
}
```
An alternative would be to implicitly bind coroutine arguments to the same names after each yield point:
```rust
|a1| {
    yield x;  // actually means let (a1,) = yield x; 
}
```

# Unresolved questions

## Should coroutines be introduced with a special keyword,- to distinguish them from regular closures?  
For example, ```coro |a, b| { ... }```

A: Technically, there is not need for that, the coroutine-ness may be inferred from the presence of
**yield** expressions (as was done in Python).

## Should the "[No borrows across yield points](#no-borrows-across-yield-points)" rule be relaxed when coroutine environment is "pinned"?
If a coroutine yields references borrowing from the coroutine's environment, the environment would become "pinned", 
as external code would know about the outstanding borrow and will not permit coroutine environment to be moved.
In other words, the coroutine closure would satisfy this bound:
```rust
F: FnMut(...)->CoResult<&'a T + 'a, ...>
```
Under these circumstances, borrows could be allowed to live across yield points.

# Appendix: Motivating Examples

### Iterators

With the help of this adapter,
```rust
impl<T,F> Iterator for F where F: FnMut()->CoResult<T,()> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self() {
            Yield(x) => Some(x),
            Return(..) => None
        }
    }
}
```
... we can implement a collection iterator in "procedural" style:
```rust
impl<T> [T] {
    ...
    fn iter(&'a self) -> impl Iterator<T> + 'a {
        || {
            let mut i = 0;
            while i < self.len() {
                yield self[i];
            }
        }
    }
}
```

### Double-ended iterators

Similarly, a double-ended iterator can be implemented as follows:
```rust
enum IterEnd {
    Head,
    Tail
}

impl<T,F> Iterator for F where F: FnMut(IterEnd) -> CoResult<T,()> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self(IterEnd::Tail) {
            Yield(x) => Some(x),
            Return(..) => None
        }
    }
}

impl<T,F> DoubleEndedIterator for F where F: FnMut(IterEnd)->CoResult<T,()> {
    fn next_back(&mut self) -> Option<T> {
        match self(IterEnd::Head) {
            Yield(x) => Some(x),
            Return(..) => None
        }
    }
}

impl<T> [T] {
    fn iter(&'a self) -> impl DoubleEndedIterator<T> + 'a {
        |which_end: mut IterEnd| {
            let mut i = 0;
            let mut j = self.len();
            while i < j {
                match which_end {
                    Tail => {
                         which_end = yield self[i];
                         i += 1;
                    },
                    Head => {
                        j -= 1;
                        which_end = yield self[j];
                    }
                }
            }
        }
    }
}
```

### Asynchronous I/O

This is an implementation of [`tokio_core::io::copy`](https://github.com/tokio-rs/tokio-core/blob/0588204048f7562b9f31dc4bd5afba19bd5c7f69/src/io/copy.rs#L32)
using a coroutine:

```rust
use futures::future::Future;
use std::io;

pub fn copy<R, W>(mut reader: R, mut writer: W) -> impl Future<Item=usize, Error=io::Error>
    where R: io::Read, W: io::Write
{
    || {
        let mut total: usize = 0;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = await_nb!(reader.read(&mut buffer))?;
            if read == 0 { return Ok(total) }
            total += read;
            let mut written = 0;
            while written < read {
                written += await_nb!(writer.write(&buffer[written..read]))?;
            }
        }
    }
}

////// Syntax sugar and plumbing

impl<T,E,F> Future for F where F: FnMut() -> CoResult<(), Result<T,E>> {
    type Item = T;
    type Error = E;
    fn poll(&mut self) -> Poll<T, E> {
        match self() {
            Yield(()) => Ok(Async::NotReady),
            Return(Ok(r)) => Ok(Async::Ready(r)),
            Return(Err(e)) => Err(e),
        }
    }
}

// Awaiter for Future's
macro_rules! await {
    ($e:expr) => ({
        let mut future = $e;
        loop {
            match future.poll() {
                Ok(Async::Ready(r)) => break Ok(r),
                Ok(Async::NotReady) => yield,
                Err(e) => break Err(e.into()),
            }
        }
    })
}

// Awaiter for non-blocking I/O.
macro_rules! await_nb {
    ($e:expr) => (
        loop {
            match $e {
                Ok(r) => break Ok(r),
                Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => yield,
                Err(e) => break Err(e.into(),
            }
      )
}

// `await` and `await_nb` can probably be unified using sufficiently advanced abstraction.

```
