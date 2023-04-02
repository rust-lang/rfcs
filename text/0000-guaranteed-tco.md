- Feature Name: guaranteed_tco
- Start Date: 2023-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature allows guaranteeing that function calls are tail-call optimized (TCO) via the `become` keyword. If this guarantee can not be provided by the compiler an error is generated instead. The check for the guarantee is done by verifying that the candidate function call follows several restrictions such as tail position and a function signature that exactly matches the calling function (it might be possible to loosen the function signature restriction in the future).

This RFC discusses a minimal version that restricts function signatures to be exactly matching the calling function. It is possible that some restrictions can be removed with more experience of the implementation and usage of this feature. Also note that the current proposed version does not support general tail call optimization, this likely requires some more changes in Rust and the backends.

# Motivation
[motivation]: #motivation

While opportunistic TCO is already supported there currently is no way to natively guarantee TCO. This optimization is interesting for two general goals. One goal is to do function calls without adding a new stack frame to the stack, this mainly has semantic implications as for example recursive algorithms can overflow the stack without this optimization. The other goal is to, in simple words, replace `call` instructions by `jmp` instructions, this optimization has performance implications and can provide massive speedups for algorithms that have a high density of function calls.

Note that workarounds for the first goal exist by using so called trampolining which limits the stack depth. However, while this functionality is provided by several crates, a inclusion in the language can provide greater adoption of a more functional programming style.

For the second goal no guaranteed method exists, so if TCO is performed depends on the specific structure of the code and the compiler version. This can result in TCO no longer being performed if non-semantic changes to the code are done or the compiler version changes.

Some specific use cases that are supported by this feature are new ways to encode state machines and jump tables, allowing code to be written in a continuation-passing style, recursive algorithms to be guaranteed TCO, and faster interpreters. One common example for the usefulness of tail-calls in C is improving performance of Protobuf parsing [blog](https://blog.reverberate.org/2021/04/21/musttail-efficient-interpreters.html), which would then also be possible in Rust.


# TODO Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Introducing new named concepts. 
The `become` keyword can be used at the same locations as the `return` keyword, however, only a *simple* function call can take the place of the argument. That is supported are calls such as `become foo()`, `become foo(a)`, `become foo(a, b)`, however, **not** supported are calls that contain or are part of a larger expression such as `become foo() + 1`, `become foo(1 + 1)`, `become foo(bar())` (though this may be subject to change). Additionally, there is a further restriction on the tail-callable functions: the function signature must exactly match that of the calling function (a restriction that might be loosened in the future). 

## Explaining the feature largely in terms of examples.
Now on to some examples. Starting with how `return` and `become` differ, and some potential pitfalls. 
TODO add usecases

### The difference between `return` and `become`
One essential difference to `return` is that `become` drops function local variables **before** the function call instead of after. So the following function ([original example](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1136728427)):
```rust
fn x() {
    let a = Box::new(());
    let b = Box::new(());
    become y(a)
}
```

Will be desugared in the following way:
```rust
fn x() {
    let a = Box::new(());
    let b = Box::new(());
    let _tmp = a;
    drop(b);
    become y(_tmp)
}
```

This early dropping allows to avoid many complexities associated with deciding if a call can be TCO, instead the heavy lifting is done by the borrow checker and a lifetime error will be produced if references to local variables are passed to the called function. To be clear a reference to a local variable could be passed if instead of `become` the call would be done with `return y(a);` (or equivalently `y(a)`), indeed this difference between the handling of local variables is also the main difference between `return` and `become`.

### Omission of the `become` keyword causes the call to be `return` instead.
([original example](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-278988088))

```rust
fn foo(x: i32) -> i32 {
    if x % 2 {
        let x = x / 2;
        // one branch uses `become`
        become foo(new_x);
    } else {
        let x = x + 3;
        // the other does not
        foo(x) // == return foo(x);
    }
}
```

This is a potential source of confusion, indeed in a function language where every call is expected to be TCO this would be quite unexpected. (Maybe in functions that use `become` a lint should be applied that enforces usage of either `return` or `become`.)


### Alternating `become` and `return` calls
([original example](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-279062656))

```rust
fn foo(n: i32) {
    // ups! we forgot become!
    return bar(n); // or alternatively: `bar(n)`
}

fn bar(n: i32) {
    become foo(n);
}
```

Here one function uses `become` the other `return`, this is another potential source of confusion. This mutual recursion would eventual overflow the stack. As mutual recursion can also happen across more functions, `become` needs to be used consistently in all functions if TCO should be guaranteed. (Maybe it is also possible to create a lint for these use-cases as well.)

<!-- TODO
```rust
fn sum_list(data: Vec<u64>, mut offset: usize, mut accum: u64) -> u64 {
    if offset < data.len() {
        accum += data[offset];
        offset += 1;
        become sum_list(data, offset, accum)
    } else {
        accum
    }
}
``` -->


## Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
This feature is only useful for some specific algorithms, where it can be essential, though it might also create a push towards a more functional programming style in Rust. In general this feature is probably unneeded for most Rust programmers, Rust has been getting on fine without this feature for most applications. As a result it impacts only those few Rust programmers that require TCO provided by this feature.


## If applicable, provide sample error messages, deprecation warnings, or migration guidance.
TODO Error messages

As this is a independent new feature there should be no need for deprecation warnings.

Regarding migration guidance, it might be interesting to provide a lint that indicates that a trivial transformation from `return` to `become` can be done for function calls where requisites are already fulfilled. However, this lint might be confusing and noisy without too much of a benefit, especially if TCO is already done without `become`.


## If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
For new Rust programmers this feature should probably be introduced late into the learning process, it is not a required feature and only useful for niche problems. So it should be taught similarly as to programmers that already know Rust. It is likely enough to provide a description of the feature, explain TCO, compare the differences to `return`, and give examples of possible use-cases and mistakes.


## Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?
As this feature introduces a new keyword and is independent of existing code it has no impact on existing code. For code that does use this feature, it is required that a programmer understands the differences between `become` and `return`, it is difficult to judge how big this impact is without an initial implementation. One difference, however, is in debugging code that uses `become`. As the stack is not preserved, debugging context is lost which likely makes debugging more difficult. That is, elided parent functions as well as their variable values are not available during debugging. (Though this issue might be lessened by providing a flag to opt out of TCO, which would, however, break the semantic guarantee of creating further stack frames. This is likely an issue that needs some investigation after creating an initial implementation.)


# TODO Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

As this feature should be mostly independent from other features the main drawback lies in the implementation and maintenance effort. This feature adds a new keyword which will need to be implemented not only in Rust but also in other tooling. The main effort, however, lies in supporting this feature in the backends:
- LLVM supports a `musttail` marker to indicate that TCO should be performed [docs](https://llvm.org/docs/LangRef.html#id327). Clang which already depends on this feature, seems to only generate correct code for the x86 backend [source](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1490009983) (as of 30.03.23).
- GCC does not support a equivalent `musttail` marker.
- WebAssembly accepted tail-calls into the [standard](https://github.com/WebAssembly/proposals/pull/157/) and Cranelift is now [working](https://github.com/bytecodealliance/rfcs/pull/29) towards supporting it.

Additionally, this proposal is limited to exactly matching function signatures which will *not* allow general tail-calls, however, the work towards this initial version could be used for a more comprehensive version.

There is also a unwanted interaction between TCO and debugging. As TCO by design elides stack frames this information is lost during debugging, that is the parent functions and their local variable values are incomplete. As TCO provides a semantic guarantee of constant stack usage it is also not generally possible to disable TCO for debugging builds as then the stack could overflow. (Still maybe a compiler flag could be provided to temporarily disable TCO for debugging builds.)


# TODO Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why is this design the best in the space of possible designs?
TODO This design is the best tradeoff between implementation effort and provided functionality.

## What other designs have been considered and what is the rationale for not choosing them?

### Loop based approach

### Attribute on return

### Attribute on tail-callable functions

### Using `become` and a marker for tail-callable functions

### Custom compiler or MIR passes


## What is the impact of not doing this?
- https://github.com/rust-lang/rust/issues/102952
- Clang has support, this feature would restore this deficit parity
- 

## If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?
While there exist libraries for a trampoline based method to avoid growing the stack, this is not enough to achieve the possible performance of real TCO, so this feature requires support by the compiler itself.


# TODO Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# TODO Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# TODO Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
