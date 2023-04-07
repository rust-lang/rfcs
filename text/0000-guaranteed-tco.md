- Feature Name: guaranteed_tco
- Start Date: 2023-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This feature provides a guarantee that function calls are tail-call optimized via the `become` keyword. If this
guarantee can not be provided by the compiler a compile time error is generated instead.

# Motivation
[motivation]: #motivation

While opportunistic tail-call optimization (TCO) is already supported there currently is no way to guarantee TCO. This
guarantee is interesting for two general goals. One goal is to do function calls without growing the stack, this mainly
has semantic implications as recursive algorithms can overflow the stack without this optimization.  The other goal is
to, in simple words, replace `call` instructions by `jmp` instructions, this optimization has performance implications
and can provide massive speedups for algorithms that have a high density of function calls.

Note that workarounds for the first goal exist by using trampolining which limits the stack depth. However, while this
functionality can be provided as a library, inclusion in the language can provide greater adoption of a more functional
programming style.

For the second goal no guaranteed method exists. The decision if TCO is performed depends on the specific code and the
compiler version. This can result in TCO surprisingly no longer being performed due to small changes to the code or a
change of the compiler version, see this [issue](https://github.com/rust-lang/rust/issues/102952) for an example.

Some specific use cases that are supported by this feature are new ways to encode state machines and jump tables,
allowing code to be written in a continuation-passing style, recursive algorithms to be guaranteed TCO, or guaranteeing
significantly faster interpreters / emulators. One common example of the usefulness of tail calls in C is improving
performance of Protobuf parsing as described in this [blog post](https://blog.reverberate.org/2021/04/21/musttail-efficient-interpreters.html), this approach would then also be possible in Rust.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation
<!--
Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms. -->
Pretending this RFC has already been accepted into Rust, it could be explained to another Rust programmer as follows.

## Introducing new named concepts.
Rust now supports a way to guarantee tail call optimization (TCO), this is interesting for two groups of programmers
those that want to use recursive algorithms and those that want to create highly optimized code. Note that using this
feature can have some difficulties as there are several requirements on functions where TCO can be performed.

TCO provides a way to call functions without creating a new stack frame, instead, the stack frame of the calling
function is reused. This is only possible if the functions have a similar enough stack layout in the first place, this
layout is based on the calling convention, and arguments as well as return types (the function signature in short).
Currently, all of these need to match exactly otherwise an error will be thrown during compilation.

Reusing the stack frame has two effects: One is that the stack will no longer grow, allowing unlimited nested function
calls, if all are TCO'ed. The other is that creating a new stack frame is actually quite expensive, especially for code
with a high density of function calls, so reusing the stack frame can lead to massive performance improvements.

To guarantee TCO the `become` keyword can be used instead of the `return` keyword (and only there). However, only a
"plain" function or method call can take the place of the argument. That is supported are calls such as `become foo()`,
`become foo(a)`, `become foo(a, b)`, however, **not** supported are calls that contain or are part of a larger
expression such as `become foo() + 1`, `become foo(1 + 1)`, `become foo(bar())` (though this may be subject to change).
Additionally, as already said the function signature must exactly match that of the calling function (a restriction that
might also be loosened a bit in the future). 

## Examples
Now on to some examples. Starting with how `return` and `become` differ, two example use cases, and some potential
pitfalls. 

### The difference between `return` and `become`
The essential difference to `return` is that `become` drops function local variables **before** the function call
instead of after. So the following function ([original example](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1136728427)):
```rust
fn x() {
    let a = Box::new(());
    let b = Box::new(());
    become y(a);
}
```

Will be desugared in the following way:
```rust
fn x() {
    let a = Box::new(());
    let b = Box::new(());
    let _tmp = a;
    drop(b);
    become y(_tmp);
}
```


This early dropping allows the compiler to avoid many complexities associated with deciding if a call can be TCO,
instead the heavy lifting is done by the borrow checker and a lifetime error will be produced if references to local
variables are passed to the called function. To be clear a reference to a local variable could be passed if instead of
`become` the call would be done with `return y(a);` (or equivalently `y(a)`), indeed this difference between the
handling of local variables is also the main difference between `return` and `become`.

### Use Case 1: Recursive Algorithm
As a possible use case let us take a look at creating the sum over a `Vec`. Admittedly an unusual example for Rust as
this is usually done with iteration. Though, this is kind of the point, without TCO this example can overflow the stack.

```rust
fn sum_list(data: Vec<u64>, mut offset: usize, mut accum: u64) -> u64 {
    if offset < data.len() {
        accum += data[offset];
        offset += 1;
        become sum_list(data, offset, accum); // <- become here
    } else {
        // Note that this would be a `return accum;`
        accum
    }
}
```


### Use Case 2: Interpreter
In an interpreter the usual loop is to get an instruction, match on that instruction to find the corresponding function, **call** that function, and finally return to the loop to get the next instruction. (This is a simplified example.)

```rust
fn exec_instruction(mut self) {
    loop {
        let next_instruction = self.read_instr(); // this call can be inlined
        match next_instruction {
            Instruction::Foo => self.execute_instruction_foo(),
            Instruction::Bar => self.execute_instruction_bar(),
        }
    }
}
```

This example can be turned into the following code, which no longer does any calls and instead just uses jump instructions. (Note that this example might not be the optimal way to use `become`.)

```rust
fn execute_instruction_foo(mut self) {
    // foo things ...

    become self.next_instruction();
}

fn execute_instruction_bar(mut self) {
    // bar things ...

    become self.next_instruction();
}

fn next_instruction(mut self) {
    let next_instruction = self.read_instr(); // this call can be inlined
    match next_instruction {
        Instruction::Foo => become self.execute_instruction_foo(),
        Instruction::Bar => become self.execute_instruction_bar(),
    }
}
```

### Omission of the `become` keyword causes the call to be `return` instead.
([original example](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-278988088))

This is a potential source of confusion, indeed in a function language where every call is expected to be TCO this would be quite unexpected. (Maybe in functions that use `become` a lint should be applied that enforces usage of either `return` or `become` in functions where at least one `become` is used.)

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

### Alternating `become` and `return` calls
([original example](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-279062656))

Here one function uses `become` the other `return`, this is another potential source of confusion. This mutual recursion
would eventual overflow the stack. As mutual recursion can also happen across more functions, `become` needs to be used
consistently in all functions if TCO should be guaranteed. (Maybe it is also possible to create a lint for these
use cases as well.)

```rust
fn foo(n: i32) {
    // oops, we forgot become ..
    return bar(n); // or alternatively: `bar(n)`
}

fn bar(n: i32) {
    become foo(n);
}
```


## Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
This feature is only useful for some specific algorithms, where it can be essential, though it might also create a push
towards a more functional programming style in Rust. In general this feature is probably unneeded for most Rust
programmers, Rust has been getting on fine without this feature for most applications. As a result it impacts only those
few Rust programmers that require TCO provided by this feature.


## If applicable, provide sample error messages, deprecation warnings, or migration guidance.
(TODO Error messages once an initial implementation exists)

There should be no need for deprecation warnings.

Regarding migration guidance, it might be interesting to provide a lint that indicates that a trivial transformation
from `return` to `become` can be done for function calls where all requisites are already fulfilled. However, this lint
might be confusing and noisy.


## If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
For new Rust programmers this feature should probably be introduced late into the learning process, it requires
understanding some advanced concepts and the current use cases are likely to be niche. So it should be taught similarly
as to programmers that already know Rust. It is likely enough to description the feature, explain TCO, compare the
differences to `return`, and give examples of possible use cases and mistakes.


## Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?
As this feature introduces a new keyword and is independent of existing code it has no impact on existing code. For code
that does use this feature, it is required that a programmer understands the differences between `become` and `return`,
it is difficult to judge how big this impact is without an initial implementation. One difference, however, is in
debugging code that uses `become`. As the stack is not preserved, debugging context is lost which likely makes debugging
more difficult. That is, elided parent functions as well as their variable values are not available during debugging.
(Though this issue might be lessened by providing a flag to opt out of TCO, which would, however, break the semantic
guarantee of not creating stack frames. This is likely an issue that needs some investigation after creating an initial
implementation.)


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
<!-- This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work. -->
This explanation is mostly based on a [previous RFC](https://github.com/DemiMarie/rfcs/blob/become/0000-proper-tail-calls.md#detailed-design)
though is more restricted as the current RFC does not target general tail calls anymore.

The goal of this RFC is to describe a first implementation that is already useful while providing a basis to explore
possible ways to relax the requirements when TCO can be guaranteed.

## Syntax
[syntax]: #syntax

A guaranteed TCO is indicated by using the `become` keyword in place of `return`. The `become` keyword is already
reserved, so there is no backwards-compatibility break. The `become` keyword must be followed by a plain function call
or method calls, that is supported are calls like: `become foo()`, `become foo(a)`, `become foo(a, b)`, and so on, or
`become foo.bar()` with plain arguments. Neither the function call nor any arguments can be part of a larger expression
such as `become foo() + 1`, `become foo(1 + 1)`, `become foo(bar())`. Additionally, there is a further restriction on
the tail-callable functions: the function signature must exactly match that of the calling function. 

Invocations of overloaded operators with at least one non-primitive argument were considered as valid targets, but were
rejected on grounds of being too error-prone. In any case, these can still be called as methods.

## Type checking
[typechecking]: #typechecking
A `become` statement is type-checked like a `return` statement, with the added restriction of exactly matching the
function signatures between caller and callee. Additionally, the caller and callee **must** use the same calling
convention.

## Borrowchecking and Runtime Semantics
[semantics]: #semantics
A `become` expression acts as if the following events occurred in-order:

1. All variables that are being passed by-value are moved to temporary storage.
2. All local variables in the caller are destroyed according to usual Rust semantics. Destructors are called where
   necessary. Note that values moved from in step 1 are _not_ dropped.
3. The caller's stack frame is removed from the stack.
4. Control is transferred to the callee's entry point.

This implies that it is invalid for any references into the caller's stack frame to outlive the call. The borrow checker ensures that none of the above steps will result in the use of a value that has gone out of scope.

As `become` is always in a tail position (due to being used in place of `return`), this requirement for TCO is already
fulfilled.

Example:
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

## Implementation
[implementation]: #implementation

A now six years old implementation for the earlier mentioned
[RFC](https://github.com/DemiMarie/rfcs/blob/become/0000-proper-tail-calls.md) can be found at
[DemiMarie/rust/tree/explicit-tailcalls](https://github.com/DemiMarie/rust/tree/explicit-tailcalls).
A new implementation is planned as part of this RFC.

The parser parses `become` exactly how it parses the `return` keyword. The difference in semantics is handled later.

During type checking, the following are checked:

1. The target of the tail call is, in fact, a simple call.
2. The target of the tail call has the proper ABI.

Later phases in the compiler assert that these requirements are met.

New nodes are added in HIR and THIR to correspond to `become`. In MIR, the function call is checked that:
1. The returned value is directly returned.
2. There are no cleanups.
3. The basic block being branched into has length zero.
4. The basic block being branched into terminates with a return.

If these conditions are fulfilled the function call and the `become` are merged into a `TailCall` MIR node,
this guarantees that nothing can be inserted between the call and `become`. Additionally, this node indicates
the TCO requirement for the call which is then propagated to the corresponding backend. In the backend,
there is an additional check if TCO can be performed.

Should any check during compilation not pass a compiler error should be issued.


# Drawbacks
[drawbacks]: #drawbacks
<!-- Why should we *not* do this? -->
As this feature should be mostly independent of other features the main drawback lies in the implementation and
maintenance effort. This feature adds a new keyword which will need to be implemented not only in Rust but also in other
tooling. The primary effort, however, lies in supporting this feature in the backends:
- LLVM supports a `musttail` marker to indicate that TCO should be performed [docs](https://llvm.org/docs/LangRef.html#id327). Clang which already depends on this feature, seems to only generate correct code for the x86 backend [source](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1490009983) (as of 30.03.23).
- GCC does not support an equivalent `musttail` marker.
- WebAssembly accepted tail-calls into the [standard](https://github.com/WebAssembly/proposals/pull/157/) and Cranelift is now [working](https://github.com/bytecodealliance/rfcs/pull/29) towards supporting it.

Additionally, this proposal is limited to exactly matching function signatures which will *not* allow general tail-calls, however, the work towards this initial version is likely to be useful for a more comprehensive version.

There is also an unwanted interaction between TCO and debugging. As TCO by design elides stack frames this information is lost during debugging, that is the parent functions and their local variable values are incomplete. As TCO provides a semantic guarantee of constant stack usage it is also not generally possible to disable TCO for debugging builds as then the stack could overflow. (Still maybe a compiler flag could be provided to temporarily disable TCO for debugging builds. As suggested [here](https://github.com/rust-lang/rfcs/pull/3407/files#r1159817279), another option would be special support for `become` by a debugger. With this support the debugger would keep track of the N most recent calls providing at least some context to the bug.)


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why is this design the best in the space of possible designs?
This design is the best tradeoff between implementation effort and functionality, while also offering a good starting
point toward further exploration of a more general implementation. To expand on this, compared to other options
creating a function local scope with the use of `become` greatly reduces implementation effort. Additionally, limiting
tail-callable functions to those with exactly matching function signatures enforces a common stack layout across all
functions. This should in theory, depending on the backend, allow tail calls to be performed without any stack
shuffling, indeed it might even be possible to do so for indirect calls or external functions.

## What other designs have been considered and what is the rationale for not choosing them?
There are some designs that either can not achieve the same performance or functionality as the chosen approach. Though most other designs evolve around how to mark what should be a tail-call or marking what functions can be tail called. There is also the possibility of providing support for a custom backend (e.g. LLVM) or MIR pass.

### Trampoline based Approach
There could be a trampoline-based approach
([comment](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-326952763)) that can fulfill the semantic guarantee
of using constant stack space, though they can not be used to achieve the performance that the chosen design is capable
of. Additionally, functions need to be known during compile time for these approaches to work.

### Principled Local Goto
One alternative would be to support some kind of local goto natively, indeed there exists a
[pre-RFC](https://internals.rust-lang.org/t/pre-rfc-safe-goto-with-value/14470/9?u=scottmcm) ([comment](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1458604986)). This design should be able to achieve the same performance and stack usage, though it seems to be quite difficult to implement and does not seem to be as flexible as the chosen design (especially regarding indirect calls / external functions).

### Attribute on Function Declaration
One alternative is to mark a group of functions that should be mutually tail-callable [example](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-1161525527) with some follow up [discussion](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-1185828948).

The goal behind this design is to allow TCO of functions that do not have exactly matching function signatures, in
theory, this just requires that tail-called functions are callee cleanup, which is a mismatch to the default calling
convention used by Rust. To limit the impact of this change all functions that should be TCO-able should be marked with
an attribute.

While quite noisy it is also less flexible than the chosen approach. Indeed TCO is a property of the call and not a
function, sometimes a call should be guaranteed to be TCO and sometimes not, marking a function would be less flexible.

### Attribute on `return`
One alternative could be to use an attribute instead of the `become` keyword for function calls. To my knowledge, this would be the first time an attribute would be allowed for a call. Example:

```rust
fn a() {
    become b();
    // or
    #[become]
    return b();
}
```

This alternative mostly comes down to taste (or bikeshedding) and `become` was chosen as it is already reserved and
shorter to write.

### Custom compiler or MIR passes
One more distant alternative would be to support a custom compiler or MIR pass so that this optimization can be done externally. While supported for LLVM [Zulip](https://rust-lang.zulipchat.com/#narrow/stream/187780-t-compiler.2Fwg-llvm/topic/.E2.9C.94.20Running.20Custom.20LLVM.20Pass/near/320275483), for MIR this is not supported [discussion](https://internals.rust-lang.org/t/mir-compiler-plugins-for-custom-mir-passes/3166/10).

This would be an error-prone and unergonomic approach to solving this problem.


## What is the impact of not doing this?
> Rust's goal is to empower everyone to build reliable and efficient software.
([source](https://blog.rust-lang.org/inside-rust/2022/04/04/lang-roadmap-2024.html))

This feature provides a crucial optimization for some low-level code. It seems that without this feature there is a big
incentive for developers of those specific applications to use other system-level languages that can perform TCO.

Additionally, this feature enables recursive algorithms that require TCO, which would provide better support for
functional programming in Rust. 


## If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?
While there exist libraries for a trampoline-based method to avoid growing the stack, this is not enough to achieve the
possible performance of real TCO, so this feature requires support from the compiler itself.


# Prior art
[prior-art]: #prior-art
<!-- 
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
-->
Functional languages (such as OCaml, SML, Haskell, Scheme, and F#) usually depend on proper tail calls as a language
feature, which requires guaranteed TCO. For system-level languages, guaranteed TCO is usually wanted but implementation
effort is a common reason this is not yet done. Even languages with managed code such as .Net or ECMAScript (as per the
standard) also support guaranteed TCO, again performance and resource usage were the main motivators for their
implementation.

See below for a more detailed description on select compilers and languages.


## Clang
Clang, as of April 2021, does offer support for a `musttail` attribute on `return` statements in both C and C++. This
functionality is enabled by the support in LLVM, which should also be the first backend for an initial implementation in
Rust.

It seems this feature is received with "excitement" by those that can make use of it, a popular example of its usage is to improve [Protobuf parsing speed](https://blog.reverberate.org/2021/04/21/musttail-efficient-interpreters.html). However, one issue is that it is not very portable and there still seems to be some problem with the [implementation](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1490009983).


For a more detailed description see this excerpt from the description of the feature, taken from the [implementation](https://reviews.llvm.org/rG834467590842):

>  Guaranteed tail calls are now supported with statement attributes
>  ``[[clang::musttail]]`` in C++ and ``__attribute__((musttail))`` in C. The
>  attribute is applied to a return statement (not a function declaration),
>  and an error is emitted if a tail call cannot be guaranteed, for example if
>  the function signatures of caller and callee are not compatible. Guaranteed
>  tail calls enable a class of algorithms that would otherwise use an
>  arbitrary amount of stack space.
>
> If a ``return`` statement is marked ``musttail``, this indicates that the
>  compiler must generate a tail call for the program to be correct, even when
>  optimizations are disabled. This guarantees that the call will not cause
>  unbounded stack growth if it is part of a recursive cycle in the call graph.
>
> If the callee is a virtual function that is implemented by a thunk, there is
>  no guarantee in general that the thunk tail-calls the implementation of the
>  virtual function, so such a call in a recursive cycle can still result in
>  unbounded stack growth.
>
> ``clang::musttail`` can only be applied to a ``return`` statement whose value
> is the result of a function call (even functions returning void must use
> ``return``, although no value is returned). The target function must have the
> same number of arguments as the caller. The types of the return value and all
> arguments must be similar according to C++ rules (differing only in cv
> qualifiers or array size), including the implicit "this" argument, if any.
> Any variables in scope, including all arguments to the function and the
> return value must be trivially destructible. The calling convention of the
> caller and callee must match, and they must not be variadic functions or have
> old style K&R C function declarations.

There is also a [proposal](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n2920.pdf) for the [C Standard](https://www.open-std.org/JTC1/SC22/WG14/) outlining some limitations for Clang.
> Clang requires the argument types, argument number, and return type to be the same between the
> caller and the callee, as well as out-of-scope considerations such as C++ features and the calling
> convention. Implementor experience with Clang shows that the ABI of the caller and callee must be
> identical for the feature to work; otherwise, replacement may be impossible for some targets and
> conventions (replacing a differing argument list is non-trivial on some platforms).


## GCC
GCC does not support a feature equivalent to Clang's `musttail`, there also does not seem to be push to implement it ([pipermail](https://gcc.gnu.org/pipermail/gcc/2021-April/235882.html)) (as of 2021). However, there also exists a experimental [plugin](https://github.com/pietro/gcc-musttail-plugin) for GCC last updated in 2021.


## Zig
Zig provides separate syntax to allow more flexibility than normal function calls. There are options for async calls, inlining, compile time evaluation of the called function, and to enforce TCO on the call.
([source](https://ziglang.org/documentation/master/#call))
```zig
const expect = @import("std").testing.expect;

test "noinline function call" {
    try expect(@call(.auto, add, .{3, 9}) == 12);
}

fn add(a: i32, b: i32) i32 {
    return a + b;
}
```

(TODO: What is the community sentiment regarding this feature? Except for some bug reports I did not find anything.)

## Carbon
As per this [issue](https://github.com/carbon-language/carbon-lang/issues/1761) it seems providing TCO is of interest even if the implementation is difficult


## .Net
The .Net JIT does support TCO as of 2020, a main motivator for this feature was improving performance.
[Pull Request](https://github.com/dotnet/runtime/pull/341) ([Issue](https://github.com/dotnet/runtime/issues/2191))
> This implements tailcall-via-help support for all platforms supported by
> the runtime. In this new mechanism the JIT asks the runtime for help
> whenever it realizes it will need a helper to perform a tailcall, i.e.
> when it sees an explicit tail. prefixed call that it cannot make into a
> fast jump-based tailcall.


## ECMA Script / JS
https://github.com/rust-lang/rfcs/pull/1888#issuecomment-368204577 (Feb, 2018)
> Technically the ES6 spec mandates tail-calls, but the situation in reality is more complicated than that.
>
> The only browser that actually supports tail calls is Safari (and Webkit). And the Edge team has said that it's unlikely that they will implement tail calls (for similar reasons as Rust: they currently use the Windows ABI calling convention, which doesn't work well with tail calls).
>
> Therefore, tail calls in JS is a very controversial thing, even to this day
>
> Just to be clear, the Edge team is against implicit tail-calls for all functions, but they're in favor of tail-calls-with-an-explicit-keyword (similar to this RFC).


An unofficial summary of the ECMA Script/ Javascript proposal for tail call/return
https://github.com/carbon-language/carbon-lang/issues/1761#issuecomment-1198672079 (Jul, 2022)

# Unresolved questions
[unresolved-questions]: #unresolved-questions
<!--
- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC? -->

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
    - The main uncertainties are regarding the exact restrictions on when backends can guarantee TCO, this RFC is intentionally strict to try and require as little as possible from the backends.
    - One point that needs to be decided is if TCO should be a feature that needs to be required from all backends or if it can be optional.
    - Another point that needs to be decided is if TCO is supported by a backend what exactly should be guaranteed? While the guarantee that there is no stack growth should be necessary, should performance (as in transforming `call` instructions into `jmp`) also be guaranteed? Note that a backend that guarantees performance should do so **always** otherwise the main intent of this RFC seems to be lost.
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
    - Are all calling-convention used by Rust available for TCO with the proposed restrictions on function signatures?
    - Can the restrictions on function signatures be relaxed?
    - Can generic functions be supported?
    - Can async functions be supported? (see [here](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-1186604115) for an initial assessment)
    - Can closures be supported? (see [here](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-1186604115) for an initial assessment)
    - Can dynamic function calls be supported?
    - Can functions outside the current crate be supported, functions from dynamically loaded libraries?
    - Can functions that abort be supported?
    - Is there some way to reduce the impact on debugging?


# Future possibilities
[future-possibilities]: #future-possibilities
<!-- Think about what the natural extension and evolution of your proposal would
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
The section merely provides additional information. -->
## Helpers
It seems possible to keep the restriction on exactly matching function signatures by offering some kind of placeholder arguments to pad out the differences. For example:
```rust
foo(a: u32, b: u32) {
    // uses `a` and `b`
}

bar(a: u32, _b: u32) {
    // only uses `a`
}
```
Maybe it is useful to provide a macro or attribute that inserts missing arguments.
```rust
#[pad_args(foo)]
bar(a: u32) {
    // ...
}
```

## Functional Programming
This might be a silly idea but if guaranteed TCO is supported there could be further language extensions to make Rust
more attractive for functional programming paradigms. Though it is unclear to me how far this should be taken or what
changes exactly would be a benefit.
