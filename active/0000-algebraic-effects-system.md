- Start Date: 2014-04-07
- RFC PR #:
- Rust Issue #:

**References:**

* [Programming and Reasoning with Algebraic Effects and Dependent Types](http://eb.host.cs.st-andrews.ac.uk/drafts/effects.pdf)


# Summary

(Some of the ideas and concepts are inspired by the [effects proposal in the Rust wiki](https://github.com/mozilla/rust/wiki/Proposal-for-effects) written by [@bblum](https://github.com/bblum))

An algebraic effects system allows one to understand functions in terms of their *effects*.

The goal is to provide mechanics to describe and enforce effects that functions may or may not have. This allows one to reason about complex systems much more intuitively and confidently.

# Motivation

Safety is the cornerstone of the language (within the realms that a systems language allows &mdash; i.e., practicality). Pure functions, in languages like Haskell, don't allow any side effects, but real applications do. Things like state, communication over the network, file I/O, etc... Above all that, real systems may fail.

Haskell, for example, solves such problems with the use of monads and monad transformers. However, there are problems with using such a tool; paraphrasing the paper, Monads don't compose very well; Monad transformers can become unwieldy when many effects are being managed.

Algebraic effects presents a more appropriate tool for dealing with function or program effects. They're not as powerful; monads and monad transformers have wider utility beyond just effects.

A few examples of effects:

* Task Failure
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
effect Failure
effect IO
effect GC
effect Anything
```

The first letter of each word in effects are capitalized, the remaining letters are lowercased. `Failure`, `GC`, `Anything`, `IO`. (I'm treating acronyms as separate words)

Full list of keywords needed:

```rust
effect
forbid
trustme
```

**Note:** `wont` has been renamed to `forbid` to allow for proper grammar and `trustme` is open for improvments.

### Annotating Effects

Now comes annotating a single function with it's effects.

```rust
fn compare(a: int, b: int) -> bool effect(Failure) {
	fail!("Oops, I failed.");
	a > b
}
```

The `effect()` expression will follow the function signature including the return type. Multiple effects may be used in a single annotation for terseness.

```rust
fn alloc(a: int, b: int, c: int) -> Vec<int> effect(Failure, Alloc) {
	fail!("Oops, I failed.");
	vec![a, b, c]
}
```

In this example, we have allocated memory on the heap.

### Enforcing Annotations

These effects are great. We can effectively tell what functions *do*. However, we need to start enforcing these annotations. By default, the top-level effect `Anything` is enforced. This allows one that does not care about effects to continue along without any breaking changes. 

Enforcements use the `forbid(Effect1, ...EffectN)` syntax the same way as effects annotation.

```rust
// dont-fail.rs
fn random() effect(Failure) {
    fail!("Ooops! I actually did fail.");
}

fn safe() forbid(Failure) {
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

**Fail:**

```rust
fn check() { // effect(Failure)
	fail!("Oops. That went wrong.");
}
```

### Higher-order Types

Given the following code:

```rust
fn take(arg: ||) forbid(Failure) {
	arg();
}

fn main() {
	take(|| {
		fail!("Hello World! I failed.");
	})
}
```

We have an issue here. The enforcement of `forbid(Failure)` applies to what? 

1. The whole function and it's context. If it calls a function that fails then it also fails (and won't compile). However, I'm unsure if the compiler can take note of that.
2. The closure `arg` will need to also be marked as `forbid(Failure)`.


```rust
fn take(arg: || forbid(Failure)) forbid(Failure) {
	arg();
}

take(|| {
	fail!("oops.");
})
```

### Trust Me

Following in the footsteps of unsafe, where a user may perform unsafe behaviour and say "trust me compiler, I know what I'm doing."

A `trustme` keyword could be added to provide an override; ensuring the compiler that a particular side-effect *won't* happen. For example:

```rust
fn dosomething() trustme(forbid(Fail)) {
	// I hope you know what you're doing in here...
}
```

### User Defined Effects

I don't see a particularly strong case for custom effects that users would define.

Some problems that could occur:

* Incompatible-composability when using libraries and sharing code.
* Inability to integrate the inference required for effects as an extension.
* Besides the core effects, there really shouldn't be *that* many important ones left for the user to define. They could simply submit a patch to include it in the compiler natively.


### Use Cases

Quoting the prominent use case from the wiki proposal:

> The most obviously useful reason to have effects is that currently destructors can leak memory if they fail when a task is already unwinding (#910). With effects, we could write:

```rust
trait Drop {
    fn drop(self) forbid(Fail);
}
```


> We might also want to forbid GC in destructors, as per #6996.

> A "fantasy" reason is that, with the old borrow checker (where &mut Ts were copyable, and &mut T could be borrowed into &T only if the surrounding code was "pure"), effect inference would avoid needing to write pure explicitly on any function you wanted to call from such code, and `forbid(Mutate)` could be inferred.

> Other speculative reasons include:

> * Reasoning about concurrency nondeterminism (#3094)
> * Preventing garbage collection in performance-critical code (such as the renderer thread in Servo)
> * Preventing dynamic allocation or rescheduling in "atomic"-context > kernel code (I hear we're running in ring 0 these days)
> * Allowing users to reason about whatever arbitrary effects their > own software might have

# Alternatives

Currently, in most languages (especially C++), you're in-charge of managing the effects yourself. People typically don't design systems in terms of effects and effect containment. Rust already has concepts like lifetimes, where people would of had to traditionally manage them manually, with no safety or guarantees.

The syntax position (after the function signature but before the body) seems to be the best position, but one could also move it to append the body.

# Unresolved questions

The original wiki explains a trouble with such a system:

> There is something of a "library boundary discipline" risk here. Suppose Alice writes a library fn a() which happens not to fail, and Bob writes a fn b() forbid(Fail) that uses a(). Later Alice, who doesn't care about effects, updates her library and makes it possibly fail. This breaks Bob's code in a way akin to changing the actual type signature of a function, except the "type" is inferred, which makes it more of a surprise. This downside is unavoidable given the desire to be unobtrusive in the common case.

However, I disagree. In this case, Bob clearly doesn't want his function to fail. If Alice changes to function to fail, Bob's program **should not compile** because that's the whole point of these guarantees.

* A real issue, as the wiki explained is the use of `assert`. Should assert always have a `Fail` effect, or should it be an exception? If it fails in a destructor, then we have the memory leak issue.
* Should print/debug statements count as I/O?
* In places where the compiler cannot infer the appropriate effect (i.e., unsafe code) should the user be forced (a la lifetimes) to be explicit?
* `Fail` or `Failure` as an effect presents some difficulties. In essence, *most* things *could* cause a failure without a rigorous static stack analysis, banning memory allocation, ban recursion, etc... Thus, there should be a discussion about, if `Failure` is introduced, to what extend does it apply to? Or, perhaps it's not a useful effect because of the previously stated problems and other combination of effects should replace it.
* A comprehensive list of effects hasn't yet been made. This should be done as part of the RFC process.
* Backward-compatibility issues should be addressed. This RFC is against having any disruption to current users/code. Users shouldn't be forced to (the majority of times, i.e., everywhere except perhaps unsafe code.) annotate or forbid effects if they don't want to. This won't cause the learning curve of Rust to dramatically increase. 

This proposal hasn't yet touched on Traits and effect parameters.

---

Thanks to [@bblum](https://github.com/bblum) for the original proposal!