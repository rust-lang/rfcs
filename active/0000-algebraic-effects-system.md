- Start Date: 2014-04-07
- RFC PR #:
- Rust Issue #:

**References:**

* [Programming and Reasoning with Algebraic Effects and Dependent Types](http://eb.host.cs.st-andrews.ac.uk/drafts/effects.pdf)


# Summary

(Some of the ideas and concepts are inspired by the [effects proposal in the Rust wiki](https://github.com/mozilla/rust/wiki/Proposal-for-effects) written by [@cartazio](https://github.com/cartazio))

An algebraic effects system allows one to understand functions in terms of their *effects*.

The goal is to provide mechanics to describe and enforce effects that functions may or may not have. This allows one to reason about complex systems much more intuitively and confidently.

# Motivation

Safety is the cornerstone of the language (within the realms that a systems language allows &mdash; i.e., practicality). Pure functions, in languages like Haskell, don't allow any side effects, but real applications do. Things like state, communication over the network, file I/O, etc... Above all that, real systems may fail.

Haskell, for example, solves such problems with the use of monads and monad transformers. However, there are problems with using such a tool (See the paper for details).

Algebraic effects presents a more appropriate tool for dealing with function or program effects. They're not as powerful; monads and monad transformers have wider utility beyond just effects.

A few examples of effects:

* Task Failure
* Unsafe Code
* GC
* Dynamic Allocation
* Nondeterminism
* File I/O
* Network I/O

This would allow one to be extremely confident in their code, letting the compiler enforce even more things. 

For example, disallowing any garbage collection to be used in highly performant code, while you're using 3rd-party libraries. This might be difficult without auditing the libraries that are used (Just an example. GC isn't prominent in Rust currently and probably won't be).

The goals are as follows:

* Unobtrusive: Effect annotation shouldn't be forced upon. By default, there are no restrictions that apply to functions (such as purity). Only when you enforce a function to condone to specific effects do you annotate.
* Lightweight: Syntax and performance. These shouldn't bog the visuals of the program nor the performance of compilation. Thus, the annotations must be really terse.

# Detailed design

### Syntax

The first step is to introduce a new `effect` language keyword. This will be used to introduce new effects and to specify which effect functions allow.

```rust
effect Fail
effect Unsafe
effect IO
effect GC
effect Anything
```

The first letter of each word in effects are capitalized, the remaining letters are lowercased. `Fail`, `GC`, `Unsafe`, `IO`.

### Annotating Effects

Now comes annotating a single function with it's effects.

```rust
fn compare(a: int, b: int) -> bool effect(Fail) {
	fail!("Oops, I failed.");
	a > b
}
```

The `effect()` expression will follow the function signature including the return type. Multiple effects may be used in a single annotation for terseness.

```rust
fn alloc(a: int, b: int, c: int) -> Vec<int> effect(Fail, Alloc) {
	fail!("Oops, I failed.");
	vec![a, b, c]
}
```

In this example, we have allocated memory on the heap.

### Enforcing Annotations

These effects are great. We can effectively tell what functions *do*. However, we need to start enforcing these annotations. By default, the top-level effect `Anything` is enforced. This allows one that does not care about effects to continue along without any breaking changes. 

Enforcements use the `wont(Effect1, ...EffectN)` syntax the same way as effects annotation.

```rust
// dont-fail.rs
fn random() effect(Fail) {
    fail!("Ooops! I actually did fail.");
}

fn safe() wont(Fail) {
    random(); // Won't compile
}
```

This code above will fail to compile (this is a fake error report):

```
dont-fail.rs:7:4: 4:13 error: call to a function that can fail. 
dont-fail.rs:7         random(); // Won't compile
                       ^~~~~~~~~~~~~~~~~~~
                       You have enforced the function `safe` to not fail.
error: aborting due to previous error
```

The default enforcement that all functions receive is `Anything`.

### Inference

Annotating effects that functions may have is really powerful, but that's quite a bit of manual work. Instead, the compiler would be inferring most of the effects. Some effects that it can't infer or can't properly infer might need to be manually specified.

**Unsafe:**

```rust
fn ffi() { // effect(Unsafe)
	unsafe { call_extern_fn() }
}
```

The compiler will infer the `Unsafe` effect and annotate the function directly. No intervention needed.

Enforcing is the user's choice. This is where the grunt of the work will be done.

```rust
fn something() wont(Unsafe) {
	ffi(); // This will fail to compile.
}
```

**Fail:**

```rust
fn check() { // effect(Fail)
	fail!("Oops. That went wrong.");
}
```

### Higher-order Types

Given the following code:

```rust
fn take(arg: ||) wont(Fail) {
	arg();
}

fn main() {
	take(|| {
		fail!("Hello World! I failed.");
	})
}
```

We have an issue here. The enforcement of `wont(Fail)` applies to what? 

1. The whole function and it's context. If it calls a function that fails then it also fails (and won't compile). However, I'm unsure if the compiler can take note of that.
2. The closure `arg` will need to also be marked as `wont(Fail)`.


```rust
fn take(arg: || wont(Fail)) wont(Fail) {
	arg();
}

take(|| {
	fail!("oops.");
})
```

### Trust Me

Following in the footsteps of unsafe, where a user may perform unsafe behaviour and says "trust me compiler, I know what I'm doing."

A `trustme` keyword could be added to provide an override. For example:

```rust
fn dosomething() trustme(wont(Fail)) {
	// I hope you know what you're doing in here...
}
```


### Use Cases

Quoting the prominent use case from the wiki proposal:

> The most obviously useful reason to have effects is that currently destructors can leak memory if they fail when a task is already unwinding (#910). With effects, we could write:

```rust
trait Drop {
    fn drop(self) wont(Fail);
}
```


> We might also want to forbid GC in destructors, as per #6996.

> A "fantasy" reason is that, with the old borrow checker (where &mut Ts were copyable, and &mut T could be borrowed into &T only if the surrounding code was "pure"), effect inference would avoid needing to write pure explicitly on any function you wanted to call from such code, and wont(Mutate) could be inferred.

> Other speculative reasons include:

> * Reasoning about concurrency nondeterminism (#3094)
> * Preventing garbage collection in performance-critical code (such as the renderer thread in Servo)
> * Preventing dynamic allocation or rescheduling in "atomic"-context > kernel code (I hear we're running in ring 0 these days)
> * Allowing users to reason about whatever arbitrary effects their > own software might have

# Alternatives

Currently, in most languages (especially C++), you're in-charge of managing the effects yourself. People typically don't design systems in terms of effects and effect containment. Rust already has concepts like lifetimes, where people would of had to traditionally manage them manually, with no safety or guarantees.

The syntax position (after the function signature but before the body) seems to be the best position, but one could also move it to after the body.

# Unresolved questions

The original wiki explains a trouble with such a system:

> There is something of a "library boundary discipline" risk here. Suppose Alice writes a library fn a() which happens not to fail, and Bob writes a fn b() wont(Fail) that uses a(). Later Alice, who doesn't care about effects, updates her library and makes it possibly fail. This breaks Bob's code in a way akin to changing the actual type signature of a function, except the "type" is inferred, which makes it more of a surprise. This downside is unavoidable given the desire to be unobtrusive in the common case.

However, I disagree. In this case, Bob clearly doesn't want his function to fail. If Alice changes to function to fail, Bob's program **should not compile** because that's the whole point of these guarantees.

* A real issue, as the wiki explained is the use of `assert`. Should assert always have a `Fail` effect, or should it be an exception? If it fails in a destructor, then we have the memory leak issue.
* Should print/debug statements count as I/O?

This proposal hasn't yet touched on Traits and effect parameters.


Thanks to [@cartazio](https://github.com/cartazio) for the original proposal!