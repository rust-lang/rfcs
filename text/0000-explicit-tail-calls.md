- Feature Name: explicit_tail_calls
- Start Date: 2023-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary
While tail call elimination (TCE) is already possible via tail call optimization (TCO) in Rust, there is no way to guaranteed that a stack frame should be reused.
This RFC describes a language feature providing tail call elimination via the `become` keyword providing this guarantee.
If this guarantee can not be provided by the compiler a compile time error is generated instead.

# Motivation
[motivation]: #motivation
Tail call elimination (TCE) allows stack frames to be reused.
While TCE via tail call optimization (TCO) is already supported by Rust, as is normal for optimizations TCE will only be applied if the compiler excpects a improvement by doing so.
There is currently no way to specify that TCE should be guaranteed.
This guarantee is interesting for two general goals.
One goal is to do function calls without growing the stack, this mainly has semantic implications as recursive algorithms can overflow the stack without this optimization.
The other goal is to avoid paying the cost to create a new stack frame, replacing `call` instructions by `jmp` instructions, this optimization has performance implications and can provide massive speedups for algorithms that have a high density of function calls.

Note that workarounds for the first goal exist by using trampolining which limits the stack depth. However, while this
functionality can be provided as a library, inclusion in the language can provide greater adoption of a more functional
programming style.

For the second goal no guaranteed method exists. While TCO can have the intended effect, if it is performed depends on
the specific code and the compiler version. This can result in unexpected slow-downs after small changes to the code or
a change of the compiler version, see this [issue](https://github.com/rust-lang/rust/issues/102952) for an example.

Some specific use cases that are supported by this feature are new ways to encode state machines and jump tables,
allowing code to be written in a continuation-passing style, using recursive algorithms without the danger of
overflowing the stack, or guaranteeing significantly faster interpreters / emulators. One common example of the
usefulness of tail calls in C is improving performance of Protobuf parsing as described in this
[blog post](https://blog.reverberate.org/2021/04/21/musttail-efficient-interpreters.html),
this approach would then also be possible in Rust.


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

## Tail Call Elimination
[tail-call-elimination]: #tail-call-elimination

Rust supports a way to guarantee tail call elimination (TCE) for function calls using the `become` keyword.
If TCE is requested for a call the called function will reuse the stack frame of the calling function, assuming all requirements are fulfilled.
Note that TCE can opportunistically also be performed by Rust using tail call optimization (TCO), this will cause TCE to be used if it is deemed to be "better" (as in faster, or smaller if optimizing for space).

TCE is interesting for two groups of programmers: Those that want to use recursive algorithms,
which can overflow the stack if the stack frame is not reused; and those that want to create highly optimized code,
as creating new stack frames can be expensive.

To request TCE the `become` keyword can be used instead of `return`, and only there.
However, it is not quite so simple.
Several requirements need to be fulfilled for TCE (and TCO) to work.

The main restriction is that the argument to `become` can be simplified to a tail call,
the call is the last action that happens in the function.
Supported are calls such as `become foo()`, `become foo(a)`, `become foo(a, b)`, `become foo(1 + 1)`,
`become foo(bar())`, `become foo.method()`, or `become function_table[idx](arg)`.
Calls that are not in the tail position can **not** be used for example `become foo() + 1` is not allowed.
The function would need to be evaluated and then the addition would need to take place.

A further restriction is on the function signature of the caller and callee.
As the stack frame should be reused it needs to be similar for both functions.
The stack frame layout is based on the calling convention, arguments, as well as return types (the function signature in
short).
Currently, all of these need to match exactly.

There is a further restriction on the arguments.
As the stack frame of the calling function is replaced it is not possible to pass references to local variables.
This is the same reason why returning references to local variables is not possible.

If any of these restrictions are not met when using `become` a compilation error is thrown.

Note that using this feature can make debugging difficult.
As `become` causes the stack frame to be replaced, debugging context is lost.
Expect to no longer see any parent functions that used `become` in the stack trace,
or have access to their variable values while debugging.

<!-- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain? -->
As this feature is strictly opt-in and the `become` keyword is already reserved, this has no impact on existing code.

<!-- If applicable, provide sample error messages, deprecation warnings, or migration guidance. -->
(TODO Error messages once an initial implementation exists)

(TODO migration guidance)


## Teaching
For new Rust programmers this feature should probably be introduced late into the learning process, it requires
understanding some advanced concepts and the current use cases are likely to be niche. So it should be taught similarly
as to programmers that already know Rust.

## Examples
On to some examples. Starting with how `return` and `become` differ, two example use cases, and some potential
pitfalls. 

### The difference between `return` and `become`
[difference]: #difference
The essential difference to `return` is that `become` drops function local variables **before** the function call
instead of after. So the following function ([original example](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1136728427)):
```rust
fn x() {
    let a = Box::new(());
    let b = Box::new(());
    become y(a);
}
```

The drops will be elaborated by the compiler like this:
```rust
fn x() {
    let a = Box::new(());
    let b = Box::new(());
    drop(b); // `a` is not dropped because it is moved to the callee
    become y(a);
}
```

If we used `return` instead, the drops would happen after the call:
```rust
fn x() {
    let a = Box::new(());
    let b = Box::new(());
    let tmp = y(a);
    drop(b); // `a` is not dropped because it is moved to the callee
    return tmp;  
}
```


This early dropping allows the compiler to avoid many complexities associated with deciding if the stack frame can be
reused. Instead, the heavy lifting is done by the borrow checker, which will produce a lifetime error if references to
local variables are passed to the called function.  This is distinct from `return`, which _does_ allow references to
local variables to be passed.  Indeed, this difference in the handling of local variables is also the main difference
between `return` and `become`.

### Use Case 1: Recursive Algorithm
A simple example is the following algorithm for summing the elements of a `Vec`.  While this would usually be done with iteration in Rust, this example illustrates a simple use of `become`.  Without TCE, this example could overflow the stack.

```rust
fn sum_list(data: Vec<u64>, mut offset: usize, mut accum: u64) -> u64 {
    if offset < data.len() {
        accum += data[offset];
        offset += 1;
        become sum_list(data, offset, accum); // <- become here
    } else {
        accum // <- equivalent to `return accum;`
    }
}
```


### Use Case 2: Interpreter
In an interpreter the usual loop is to get an instruction, match on that instruction to find the corresponding function, **call** that function, and finally return to the loop to get the next instruction. (This is a simplified example.)

```rust
fn exec_instruction(mut self) {
    loop {
        let next_instruction = self.read_instr();
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

This is a potential source of confusion, indeed in a functional language where calls are expected to be TCE this would be quite unexpected. (Maybe in functions that use `become` a lint should be applied that enforces usage of either `return` or `become` in functions where at least one `become` is used.)

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
would eventually overflow the stack. As mutual recursion can also happen across more functions, `become` needs to be
used consistently in all functions if TCO should be guaranteed. (Maybe it is also possible to create a lint for these
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


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation
<!-- This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work. -->
This explanation is mostly based on the [previous RFC](https://github.com/DemiMarie/rfcs/blob/become/0000-proper-tail-calls.md#detailed-design)
though is more restricted as the current RFC does not target general tail calls anymore.

The goal of this RFC is to describe a first implementation that is already useful while providing a basis to explore
possible ways to relax the requirements for TCE.

## Syntax
[syntax]: #syntax

A function call can be specified to be TCE by using the `become` keyword in place of `return`.  The `become` keyword is
already reserved, so there is no backwards-compatibility break. The `become` keyword must be followed by a plain
function call or method calls, that is supported are calls like: `become foo()`, `become foo(a)`, `become foo(a, b)`,
and so on, or `become foo.bar()` with plain arguments. Neither the function call nor any arguments can be part of a
larger expression such as `become foo() + 1`, `become foo(1 + 1)`, `become foo(bar())`. Additionally, there is a further
restriction on the tail-callable functions: the function signature must exactly match that of the calling function. 

Invocations of overloaded operators with at least one non-primitive argument were considered as valid targets, but were
rejected on grounds of being too error-prone. In any case, these can still be called as methods.

## Type checking
[typechecking]: #typechecking
A `become` statement is type-checked like a `return` statement, with the added restriction that the function signatures of the caller and callee must match exactly. Additionally, the caller and callee **must** use the same calling
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

As `become` is always in a tail position (due to being used in place of `return`), this requirement for TCE is already
fulfilled.

See this earlier [example](#the-difference-between-return-and-become) on how become causes drops to be elaborated.

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

Should any of these checks fail a compiler error should be issued.


New nodes are added in HIR and THIR to correspond to `become`. In MIR, the function call is checked that:
1. The returned value is directly returned.
2. There are no cleanups.
3. The basic block being branched into has length zero.
4. The basic block being branched into terminates with a return.

If these conditions are fulfilled the function call and the `become` are merged into a `TailCall` MIR node,
this guarantees that nothing can be inserted between the call and `become`. Additionally, this node indicates
the request for TCE for the call which is then propagated to the corresponding backend. In the backend,
there is an additional check if TCE can be performed.

Should any of these checks fail an ICE should be issued.


# Drawbacks
[drawbacks]: #drawbacks
<!-- Why should we *not* do this? -->
As this feature should be mostly independent of other features the main drawback lies in the implementation and
maintenance effort. This feature adds a new keyword which will need to be implemented not only in Rust but also in other
tooling. The primary effort, however, lies in supporting this feature in the backends:
- LLVM supports a `musttail` marker to indicate that TCE should be performed [docs](https://llvm.org/docs/LangRef.html#id327). Clang which already depends on this feature, seems to only generate correct code for the x86 backend [source](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1490009983) (as of 30.03.23).
- GCC does seem to support an equivalent `musttail` marker, though it is only accessible via the [libgccjit API](https://gcc.gnu.org/onlinedocs/gcc-7.3.0/jit/topics/expressions.html#gcc_jit_rvalue_set_bool_require_tail_call) ([source](https://github.com/rust-lang/rfcs/pull/3407#discussion_r1160013809)).
- WebAssembly accepted tail-calls into the [standard](https://github.com/WebAssembly/proposals/pull/157/) and Cranelift is now [working](https://github.com/bytecodealliance/rfcs/pull/29) towards supporting it.

Additionally, this proposal is limited to exactly matching function signatures which will *not* allow general tail-calls, however, the work towards this initial version is likely to be useful for a more comprehensive version.

There is also an unwanted interaction between TCE and debugging. As TCE by design elides stack frames this information is lost during debugging, that is the parent functions and their local variable values are incomplete. As TCE provides a semantic guarantee of constant stack usage it is also not generally possible to disable TCE for debugging builds as then the stack could overflow. (Still maybe a compiler flag could be provided to temporarily disable TCE for debugging builds. As suggested [here](https://github.com/rust-lang/rfcs/pull/3407/files#r1159817279), another option would be special support for `become` by a debugger. With this support the debugger would keep track of the N most recent calls providing at least some context to the bug.)


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why is this design the best in the space of possible designs?
This design is the best tradeoff between implementation effort and functionality, while also offering a good starting
point toward further exploration of a more general implementation. To expand on this, compared to other options
creating a function local scope with the use of `become` greatly reduces implementation effort. Additionally, limiting
tail-callable functions to those with exactly matching function signatures enforces a common stack layout across all
functions. This should in theory, depending on the backend, allow tail calls to be performed without any stack
shuffling, indeed it is even possible to do so for indirect calls or external functions.

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

The goal behind this design is to allow TCE of functions that do not have exactly matching function signatures, in
theory, this just requires that tail-called functions are callee cleanup, which is a mismatch to the default calling
convention used by Rust. To limit the impact of this change all functions that should be TCE-able should be marked with
an attribute.

While quite noisy it is also less flexible than the chosen approach. Indeed TCE is a property of the call and not a
function, sometimes a call should be guaranteed to be TCE and sometimes not, marking a function would be less flexible.

### Attribute on `return`
One alternative could be to use an attribute instead of the `become` keyword for function calls. Example:

```rust
fn a() {
    become b();
    // or
    #[become]
    return b();
}
```

This alternative mostly comes down to taste (or bikeshedding) and `become` was chosen as it is [reserved](https://rust-lang.github.io/rfcs/0601-replace-be-with-become.html) for this use, shorter to write, and as drop order changes compared to `return` a new keyword seems warranted.

### Custom compiler or MIR passes
One more distant alternative would be to support a custom compiler or MIR pass so that this optimization can be done externally. While supported for LLVM [Zulip](https://rust-lang.zulipchat.com/#narrow/stream/187780-t-compiler.2Fwg-llvm/topic/.E2.9C.94.20Running.20Custom.20LLVM.20Pass/near/320275483), for MIR this is not supported [discussion](https://internals.rust-lang.org/t/mir-compiler-plugins-for-custom-mir-passes/3166/10).

This would be an error-prone and unergonomic approach to solving this problem.


## What is the impact of not doing this?
> Rust's goal is to empower everyone to build reliable and efficient software.
([source](https://blog.rust-lang.org/inside-rust/2022/04/04/lang-roadmap-2024.html))

This feature provides a crucial optimization for some low-level code. It seems that without this feature there is a big
incentive for developers of those specific applications to use other system-level languages that can perform TCE.

Additionally, this feature enables recursive algorithms that require TCE, which would provide better support for
functional programming in Rust. 


## If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?
While there exist libraries for a trampoline-based method to avoid growing the stack, this is not enough to achieve the
possible performance of real TCE, so this feature requires support from the compiler itself.


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
feature (TCE for general calls). For system-level languages TCE is usually wanted but implementation
effort is a common reason this is not yet done. Even languages with managed code such as .Net or ECMAScript (as per the
standard) also support TCE, again performance and resource usage were the main motivators for their
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
Zig provides separate syntax to allow more flexibility than normal function calls. There are options for async calls, inlining, compile time evaluation of the called function, or specifying TCE on the call.
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
As per this [issue](https://github.com/carbon-language/carbon-lang/issues/1761) it seems providing TCE is of interest even if the implementation is difficult


## .Net
The .Net JIT does support TCE as of 2020, a main motivator for this feature was improving performance.
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
    - The main uncertainties are regarding the exact restrictions on when backends can offer TCE, this RFC is intentionally strict to try and require as little as possible from the backends.
    - One point that needs to be decided is if TCE should be a feature that needs to be required from all backends or if it can be optional.
    - Another point that needs to be decided is if TCE is supported by a backend what exactly should be guaranteed? While the guarantee that there is no stack growth should be necessary, should performance (as in transforming `call` instructions into `jmp`) also be guaranteed? Note that a backend that guarantees performance should do so **always** otherwise the main intent of this RFC seems to be lost.
    - Migration guidance, it might be interesting to provide a lint that indicates that a trivial transformation from `return` to `become` can be done for function calls where all requisites are already fulfilled. However, this lint might be confusing and noisy. Decide on if this lint or others should be added.
    - Should a lint be added for functions that are marked to be tail call or use become. See discussion [here](https://github.com/rust-lang/rfcs/pull/3407#issuecomment-1500620309).
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
    - Are all calling-convention used by Rust available for TCE with the proposed restrictions on function signatures?
    - Can the restrictions on function signatures be relaxed?
        - One option for intra-crate direct calls is to automatically pad the arguments during compilation see [here](https://github.com/rust-lang/rfcs/pull/3407#issuecomment-1500620309). Does this have an influence on other calls? How much implementation effort is it?
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
This might be a silly idea but if TCE is supported there could be further language extensions to make Rust
more attractive for functional programming paradigms. Though it is unclear to me how far this should be taken or what
changes exactly would be a benefit.
