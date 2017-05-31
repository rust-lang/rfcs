- Start Date: (fill me in with today's date, 2014-04-24)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add "shallow" coroutines similar to Python's generators.

# Motivation

This feature would simplify implementation all sorts of code, that needs to have "push" interface,
but would rather be "pull" internally.
Examples of such code include: collection iterators, lexers, parsers, servers that handle large numbers of long-running concurrent requests, etc.

# Drawbacks

Extra language complexity?

# Detailed design

### New keywords
This proposal introduces two new Rust keywords: **coro** and **yield**.

"yield" already a reserved keyword, and while, in principle, adding "coro" can be avoided(+), I think that co-routine behavior is different enough from regular lambdas to merit a new keyword for them.

(+) We could use the regular lambda syntax and infer that one is a co-routine by the presence of **yield** in its' body, the same way it's done in Python.

### Syntax
```text
coro_expr : 'coro' [ '(' [ param_list ] ')' [ '->' type ] ] block ;
param_list : param [',' param ]* ;
param : ident [ ':' type ] ;
```
Co-routine declaration may appear in the same places as "normal" lambda functions.  Top-level functions cannot be co-routines because co-routines need an environment block, just like lambdas.

Just like normal lambdas, co-routines may close over variables of the containing function.

Co-routine body may include the same statements, as for a normal lambda function, with an addition of **yield** expression:
```text
yield_expr : 'yield' [ expr ] ;
```

### Example

```rust
fn main() {
    let coroutine = coro (a1:A1, a2:A2, a3: A3) {
        ...
        let mut i = 10
        while i >= 0 {
            ...
            let (b1,b2,b3) = yield y1;
            ...
            let (c1,c2,c3) = yield y2;
            ...
            i -= 1;
        }
        ...
        return result;
    }

    // Consumer
    let mut result = coroutine(a1, a2, a3);
    while result.is_yield() {
        // ... process result
        result = coroutine(b1, b2, b3);
    }
}
```

### Semantics
The type of the closure in the example above is ```fn(A1, A2, A3) -> CoResult<Y, R>```, where:
* A1,A2,A3 are the types of co-routine arguments,
* a tuple (A1,A2,A3) is the return type of **yield** expressions in the co-routine body,
* Y is the inferred super-type of all yielded values,
* R is the inferred super-type of all returned values (including the tail expression),
* CoResult is defined as follows:
```rust
enum CoResult<Y,R> {
    Yield<Y>,
    Return<R>
}

impl CoResult<Y,R> {
    fn is_yield() -> bool {...}
    fn is_return() -> bool {...}
    fn unwrap_yield() -> Y {...}
    fn unwrap_return() -> R {...}
}
```

The first time `coroutine` is called, execution starts at the top, and the passed parameters are assigned to a1, a2 and a3.  When execution reaches the first **yield** expression, control is returned to the caller, and the result is `Yield(y1)`.

The second time `coroutine` is called, execution resumes immediately after the last executed **yield** expression, the value of which will be a tuple of parameters passed in by the caller.

And so on.

When execution reaches a **return** statement, of falls off the end of the co-routine body, it returns for the last time, passing back returned value wrapped in Return(), i.e. `Return(result)`.

Further attempts to invoke `coroutine` shall cause a task failure.

### 'Physics' of co-routines

The above example is translated into something like this (assume for a second that Rust supports **goto** statement):
```rust
struct Closure {
    state : int;
    i : int;
    // closed-over variables of the containing function (upvars) also go here
}

impl Closure {
    pub fn call(&mut self, a1:A1, a2:A2, a3: A3) {
        match (self.state) {
            0 => goto state_0,
            1 => goto state_1,
            2 => goto state_2,
            _ => fail!("invalid state")
        }
      state_0:
        ...
        self.i = 10;
        while self.i >= 0 {
            ...
            self.state = 1;
            return Yield(y1);
          state_1:
            let (b1,b2,b3) = (a1, a2, a3);
            ...
            self.state = 2;
            return Yield(y2);
          state_2:
            let (c1,c2,c3) = (a1, a2, a3);
            ...
            self.i -= 1;
        }
        ...
        self.state = -1;
        return Return(result);
    }
}
```
And on the caller side:
```
let coroutine = ~Closure { state: 0 };

let mut result = coroutine.call(a1, a2, a3);
while result.is_yield() {
    // ... process result
    result = coroutine.call(b1, b2, b3);
}
```

### Implementation notes

I believe that for the most part Rust compilation passes may treat co-routines just like normal lambdas, and
in the livenses checking, borrow checking, type inference, etc, passes **yield** expressions may be treated similarly to function calls.

Changes in type inference pass:
* Types of \<expr\> in all **yield** expressions are sub-typed to the Y type parameter of the co-routine return value.
* Types of \<expr\> in all **return** statements are sub-typed to the R type parameter of the co-routine return value.

Changes in IR generation:
* 'state' variable is added into the closure.
* Local variables whose lifetime straddles any **yield** expression are hoisted into the closure.  Note that this only moves their storage location, lifetimes stay intact.
* A "master switch" is added at the top of the function to transfer control to the right location, according to current state.
* **yield** expressions are transformed into the equivalent of
```
self.state = <N>;
return Yield(<expr>);
state_<N>:
```

* **return** statements and the tail expression are translated into `return Return(<expr>)`

### Once-ness

Although, superficially, it would seem that co-routine closures are invoked multiple times, semantically this is not so, because  resumptions continue at the point where execution was interrupted.  In this regard co-routines would be similar to `once fn`'s and should be able to move variables out of their environment.

### Cleanup

When a "normal" lambda closure goes out of scope, Rust runs destructors for all of closure's fields.
With co-routines, liveness of locals hoisted into the closure depends on its' current state.
If a co-routine runs to completion, all is well, because locals will have been disposed of in the course of normal execution.
However when co-routine closure gets destroyed "prematurely", some extra clean-up will be needed:

```
impl Drop for Closure {
    fn drop(&mut self) {
        match (self.state) {
            1 => { /* clean-up locals alive around state_1 */ },
            2 => { /* clean-up locals alive around state_2 */ }
            ...
            _ => ()
        }
    }
}

```


# Alternatives

It is possible to implement similar functionality via monads, or, I suspect, rather, something similar to
[F# computation expressions](http://msdn.microsoft.com/en-us/library/dd233182.aspx) (because Rust has
control flow statements, which aren't functions).

Note that even in F#, the built-in sequence expressions are implemented as a state machine similar to the above;
presumably because the compiler is not Smart Enough(tm) to optimize a sphagetti of lambda functions into a
state machine.

# Unresolved questions

- For iterators we'd want to return unboxed closures.  Can we haz unboxed closures?
- What is the syntax for heap-allocated coroutines?  `~coro() {...}`?  `coro proc() {...}`?  `box coro() {...}`?

Hopefully, the impending closure reform will resolve these issues for regular lambdas, and coroutines can piggy-back on that design.


# More examples

The foregoing assumes that closure reform had resulted in syntax similar to [this](http://glaebhoerl.tumblr.com/rust_closure_types),
i.e. lambdas implement `trait Fn<Arg1, Arg2, ..., Ret>`.

### Iterators

With a help of the following adapter,
```rust
impl<T> Iterator<T> for FnMut<CoResult<T,()>> {
    fn next(&mut self) -> Option<T> {
        match self.call() {
            Yield(x) => Some(x),
            Return(*) => None
        }
    }
}
```
... we can implement collection iterator in "procedural" style:
```rust
impl<'self,T> ImmutableVector<'self, T> for &'self [T] {
    fn iter(self) -> Iterator<'self, T> {
        coro {
            let mut i = 0;
            while i < self.len() {
                yield self[i];
            }
        }
    }
}
```

### Double-ended iterators

Similarly, a double ended iterator can be implemented as follows:
```rust
enum IterEnd {
    Head,
    Tail
}

impl<T> DoubleEndedIterator<T> for FnMut<IterEnd, CoResult<T,()>> {
    fn next(&mut self) -> Option<T> {
        match self.call(Tail) {
            Yield(x) => Some(x),
            Return(*) => None
        }
    }

    fn next_back(&mut self) -> Option<T> {
        match (*self)(Head) {
            Yield(x) => Some(x),
            Return(*) => None
        }
    }
}

impl<'self,T> ImmutableVector<'self, T> for &'self [T] {
    fn iter(self) -> DoubleEndedIterator<'self, T> {
        coro(which_end: IterEnd) {
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

This presumes existence of Awaitable<T> trait, which encapsulates a pending async IO operation,
as well as existence of an event loop that select()'s on an array of Awaitable's and dispatches IO completions
to the corresponding callbacks.

(If you are familiar with C# asyncs, think `yield` == `await` and `Awaitable<T>` == `Task<T>`).

```rust

type AsyncIO<T> = FnMut< CoResult<Awaitable, IOResult<R>> >;

fn copy_async(from: AsyncReader, into: AsyncWriter, buffer_size: uint) -> AsyncIO<i64>
{
	coro {
		let mut total: i64 = 0;
		let buffer = ~[u8, ..buffer_size];
		loop {
			// AsyncReader.read_async() returns Awaitable<IOResult<i64>>
			let read_result = yield from.read_async(buffer);
			match read_result {
				Err(err) => return Err(err),
				Ok(count) => {
					total += count;
					// AsyncWriter.write_async() returns Awaitable<IOResult<()>>
					yield into.write_async(buffer.slice(0, read_count));
				}
			}
		}
		total
	}
}

fn start_async_copy() {
    ...
	event_loop.register(copy_async(from, into, 1024));
	...
}
```
