- Feature Name: explicit_tail_calls
- Start Date: 2023-04-01
- RFC PR: [rust-lang/rfcs#3407](https://github.com/rust-lang/rfcs/pull/3407)
- Rust Issue: [rust-lang/rust#112788](https://github.com/rust-lang/rust/issues/112788)

# Summary
[summary]: #summary
While tail call elimination (TCE) is already possible via tail call optimization (TCO) in Rust, there is no way to guarantee that a stack frame must be reused.
This RFC describes a language feature providing tail call elimination via the `become` keyword providing this guarantee.
If this guarantee can not be provided by the compiler a compile time error is generated instead.

# Motivation
[motivation]: #motivation
Tail call elimination (TCE) allows stack frames to be reused.
While TCE via tail call optimization (TCO) is already supported by Rust, as is normal for optimizations, TCO will only be applied if the compiler expects an improvement by doing so.
However, the compiler can't have ideal analysis and thus will not always be correct in judging if an optimization should be applied.
This RFC shows how TCE can be guaranteed in Rust.

The guarantee for TCE is interesting for two general goals.
One goal is to do function calls without growing the stack, this mainly has semantic implications as recursive algorithms can overflow the stack without this guarantee.
The other goal is to avoid paying the cost to create a new stack frame, replacing `call` instructions by `jmp` instructions, this optimization has performance implications and can provide massive speedups for algorithms that have a high density of function calls. This goal also depends on the guarantee as otherwise a subtle change or a new compiler version can have an unexpected impact on performance.

Note that workarounds for the first goal exist by using trampolining which limits the stack depth. However, while this
functionality can be provided as a library, inclusion in the language can provide greater adoption of a more functional
programming style.

For the second goal, TCO can have the intended effect, however, there is no guarantee. This can result in unexpected slow-downs, for example, as can be seen in this [issue](https://github.com/rust-lang/rust/issues/102952).

Some specific use cases that are supported by this feature are new ways to encode state machines and jump tables,
code written in a continuation-passing style, ensuring recursive algorithms do not
overflow the stack, and guaranteeing good code generation for interpreters. For a language like Rust that considers performance-oriented uses to be in scope, it is important to support these kinds of programs.

## Examples from the C/C++ ecosystem

(This section is based on this [comment](https://github.com/rust-lang/rfcs/pull/3407#issuecomment-1562094439), all credit goes to @traviscross.)

The C/C++ ecosystem already has access to guaranteed TCE via Clang's [`musttail`](https://clang.llvm.org/docs/AttributeReference.html#musttail) attribute and GCC/Clang's [computed goto](https://gcc.gnu.org/onlinedocs/gcc/Labels-as-Values.html). Based on the assumption that code which uses `musttail` or computed gotos would also use `become` in Rust, we can gauge the impact of this feature by collecting a list of example programs that would not be replicable in Rust without this RFC.

The list of programs is generated as follows: 
- GitHub was searched for [uses of `musttail`](https://github.com/search?q=%2Fclang%3A%3Amusttail%7C__attribute__%5C%28%5C%28musttail%5C%29%5C%29%2F&type=code) and [uses of computed goto](https://github.com/search?q=%2Fgoto+%5C*%5Ba-zA-Z%28%5D%2F&type=code). GitHub's search only returns five pages, so this is only a sampling.
- The most popular projects are picked and each result is checked to confirm that `musttail` or computed gotos are used.
- Additionally, for `musttail`, which was only introduced in Clang 13, projects that have comments which indicate the desire to use `musttail` once legacy compiler support can be dropped are included as well. (Of which, there are two: FreeRADIUS and Pyston).
- Some projects use `musttail` (either Clang's or LLVM's) for code generation only, these are placed in a separate section. It is noted which of these projects expose guaranteed TCE to user code. (One project, Swift, uses it both internally and for code generation.)  

The resulting list of notable projects using [`musttail`](https://clang.llvm.org/docs/AttributeReference.html#musttail):

- [Protobuf](https://github.com/protocolbuffers/protobuf/blob/755f572a6b68518bde2773d215026659fa1a69a5/src/google/protobuf/port_def.inc#L337)
- [Swift](https://github.com/apple/swift/blob/670f5d24577d2196730f08762f2e70be10363cf3/stdlib/public/SwiftShims/swift/shims/Visibility.h#L112)
- [Skia](https://github.com/google/skia/blob/bac819cdc94a0a9fc4b3954f2ea5eec4150be103/src/opts/SkRasterPipeline_opts.h#L1205) (a graphics library from Google)
- [CHERIoT RTOS](https://github.com/microsoft/cheriot-rtos/blob/3e6811279fedd0195e105eb3b7ac77db93d67ec5/sdk/core/allocator/alloc.h#L1460) (a realtime operating system with memory safety)
- [FreeRADIUS](https://github.com/FreeRADIUS/freeradius-server/blob/fb281257fb86aa83547d5dacecebc12271d091ab/src/lib/util/lst.c#L560) (a RADIUS implementation) (_planning to use_)
- Example [BPF code](https://blog.cloudflare.com/assembly-within-bpf-tail-calls-on-x86-and-arm/) from a Cloudflare blog post
- [RSM](https://github.com/rsms/rsm/blob/d539fd5f09876700c0c38758f2b4354df433dd1c/src/rsmimpl.h#L115) (a virtual computer in the form of a virtual machine)
- [Tails](https://github.com/snej/tails/blob/d3b14fcce18c542211bc1fd37e378f667fdee42f/src/core/platform.hh#L52) (a Forth-like interpreter)
- [Jasmin](https://github.com/asoffer/jasmin/blob/f035ef0752c09846331c8deb2109e4ebfce83200/jasmin/internal/attributes.h#L13) (a stack-based byte-code interpreter)
- [upb](https://github.com/classicvalues/upb/blob/2effcce774ce05d08af635ba02b1733873e73757/upb/port_def.inc#L177) (a small protobuf implementation in C)
- [wasm3](https://github.com/wasm3/wasm3/blob/1a6ca56ee1250d95363424cc3a60f8fd14f24fa7/source/m3_config_platforms.h#L86) ("the self-proclaimed fastest WebAssembly interpreter")

The resulting list of notable projects using [computed goto](https://gcc.gnu.org/onlinedocs/gcc/Labels-as-Values.html):

- The [Linux](https://github.com/torvalds/linux/blob/933174ae28ba72ab8de5b35cb7c98fc211235096/kernel/bpf/core.c#L1678) kernel
- [PostgreSQL](https://github.com/postgres/postgres/blob/5c2c59ba0b5f723b067a6fa8bf8452d41fbb2125/src/backend/executor/execExprInterp.c#L119)
- [CPython](https://github.com/python/cpython/blob/41768a2bd3a8f57e6ce4e4ae9cab083b69817ec1/Python/ceval_macros.h#L76)
- [MicroPython](https://github.com/ksekimoto/micropython/blob/cd36298b9a8aec0872b439e6b302565f631c594d/py/vm.c#L219) (a lean Python implementation)
- [Godot](https://github.com/godotengine/godot/blob/4c677c88e918e22ad696f225d189124444f9665e/modules/gdscript/gdscript_vm.cpp#L392) (a 2D/3D game engine)
- [Ruby](https://github.com/ruby/ruby/blob/31b28b31fa5a0452cb9d5f7eee88eebfebe5b4d1/regexec.c#L2171) (they use it in their [regex](https://github.com/ruby/ruby/blob/31b28b31fa5a0452cb9d5f7eee88eebfebe5b4d1/regexec.c#L2171) engine as well as in their [interpreter](https://github.com/ruby/ruby/blob/31b28b31fa5a0452cb9d5f7eee88eebfebe5b4d1/vm_exec.h#L98))
- [HHVM](https://github.com/facebook/hhvm/blob/7b0dc442a81861ee65a2fc09afe51adf89faea70/hphp/runtime/vm/bytecode.cpp#L5690) (a PHP implementation from Facebook)

The resulting list of notable projects using [`musttail`](https://clang.llvm.org/docs/AttributeReference.html#musttail) for code generation:

- [Swift](https://github.com/apple/swift/blob/ba67156608763a58fc0dbddbc9d1ccce2dc05c02/lib/IRGen/IRGenModule.cpp#L583)
- [Zig](https://github.com/ziglang/zig/blob/5744ceedb8ea4b3e5906175033f634b17287f3ca/lib/zig.h#L110) (+ guaranteed [TCE exposed](https://ziglang.org/documentation/master/#call) to user code)
- [GHC](https://github.com/ghc/ghc/blob/994bda563604461ffb8454d6e298b0310520bcc8/rts/include/Stg.h#L372) (+ guaranteed [TCE exposed](https://wiki.haskell.org/Tail_recursion) to user code)
- [Clang](https://github.com/llvm/llvm-project/blob/59ad9c3f38c285e988072d100931bcbfb24196fb/clang/lib/CodeGen/CGCall.cpp#L544) (+ guaranteed [TCE exposed](https://clang.llvm.org/docs/AttributeReference.html#musttail) to user code)
- [Julia](https://github.com/JuliaLang/julia/blob/aea56a9d9547cff43c3bcfb3dac0fff91bd53793/src/llvm-multiversioning.cpp#L696)
- [Firefly](https://github.com/GetFirefly/firefly/blob/8e89bc7ec33cb8ffa9a60283c8dcb7ff62ead5fa/compiler/driver/src/compiler/passes/ssa_to_mlir/builder/function.rs#L1388) (a BEAM/Erlang implementation) (+ guaranteed [TCE exposed](https://www.erlang.org/doc/reference_manual/functions.html#tail-recursion) to user code)
- [MLton](https://github.com/MLton/mlton/blob/d082c4a36110321b00dc099858bb640c4d2d2c24/mlton/codegen/llvm-codegen/llvm-codegen.fun#L1405) (a Standard ML compiler) (+ guaranteed TCE exposed to user code)
- [Pyston](https://github.com/pyston/pyston/blob/6103fc013e9dd726efca9100a22be1ac08c58591/pyston/aot/aot_gen.py#L276) (a performance-optimizing JIT for Python) (_planning to use_)


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
If TCE is requested for a call, and all requirements are fulfilled, the called function will reuse the stack frame of the calling function.
The requirements, described in detail below, are checked by the compiler and a compiler error will be raised if they are not met.
Note that TCE can opportunistically also be performed by Rust using tail call optimization (TCO), this will cause TCE to be used if it is deemed to be "better" (as in faster, or smaller if optimizing for space).

TCE is interesting for two groups of programmers: Those that want to use recursive algorithms,
which can overflow the stack if the stack frame is not reused; and those that want to create highly optimized code
as creating new stack frames can be expensive.

The `become` keyword can be thought of similarly as `return` as both keywords act as the end of the current function.
The main difference is that the argument to `become` needs to be a function call.
However, there are several requirements on the called function which need to be fulfilled for TCE to be guaranteed, these are checked by the compiler.

The main restriction is that the argument to `become` is a tail call,
a call that is the last action performed in the function.
Supported are calls such as `become foo()`, `become foo(a)`, `become foo(a, b)`, `become foo(1 + 1)`,
`become foo(bar())`, `become foo.method()`, or `become function_table[idx](arg)`.
Calls that are not in the tail position can **not** be used, for example, `become foo() + 1` is not allowed.
In the example, the function would need to be evaluated and **then** the addition would need to take place.

A further restriction is on the function signature of the caller and callee.
The stack frame layout is based on the calling convention, arguments, as well as return types (the function signature in
short).
As the stack frame is to be reused it needs to be similar enough for both functions.
This requires that the function signature and calling convention of the calling and called function need to match exactly.

Additionally, there is a further restriction on the arguments.
As the stack frame of the calling function is reused, it needs to be cleaned up, so that the called function can take the space.
This is nearly identical to the clean up that happens when returning from a function,
all local variables, that are not returned or in the case of `become` used in the function call, are dropped.
For `become`, however, dropping necessarily happens before entering the called function.
As a result, it is not possible to pass references to local variables, nor will the called function "return" to the calling function.

If any of these restrictions are not met when using `become` a compilation error is thrown.

Note that using this feature can make debugging more difficult.
As `become` causes the stack frame to be reused, debugging context is lost.
Expect to no longer see any parent functions that used `become` in the stack trace,
or have access to their variable values while debugging.

<!-- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain? -->
As this feature is strictly opt-in and the `become` keyword is already reserved, this has no impact on existing code.

<!-- If applicable, provide sample error messages, deprecation warnings, or migration guidance. -->
<!-- (TODO Error messages once an initial implementation exists) -->

<!-- (TODO migration guidance) -->


## Teaching
For new Rust programmers this feature should probably be introduced late into the learning process, it requires
understanding some advanced concepts and the current use cases are likely to be niche. So it should be taught similarly
as to programmers that already know Rust.

## Examples
On to some examples. Starting with how `return` and `become` differ, two example use cases, and some potential
pitfalls. 

### The difference between `return` and `become`
[difference]: #difference
The difference to `return` is that `become` drops function local variables **before** the `become` function call
instead of after. To be more specific a `become` expression acts as if the following events occurred in-order:

1. Function call arguments are evaluated into temporary storage. If a local variable is used as a value in the arguments, it is moved.
2. All local variables in the caller are destroyed according to usual Rust semantics. Destructors are called where
   necessary. Note that values moved from step 1 are _not_ dropped.
3. The caller's stack frame is removed from the stack.
4. Control is transferred to the callee's entry point.

This implies that it is invalid for any references into the caller's stack frame to outlive the call. The borrow checker ensures that none of the above steps will result in the use of a value that has gone out of scope.

See the [example](#the-difference-between-return-and-become) below on how `become` causes drops to be elaborated.
<!-- ([original example](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1136728427)) -->

```rust
fn x(_arg_zero: Box<()>, _arg_one: ()) {
    let a = Box::new(());
    let b = Box::new(());
    let c = Box::new(());

    become y(a, foo(b));
}
```

The drops will be elaborated by the compiler like this:
```rust
fn x(_arg_zero: Box<()>, _arg_one: ()) {
    let a = Box::new(());
    let b = Box::new(());
    let c = Box::new(());

    // Move become arguments to temporary variables.
    let function_ptr = y; // The function pointer could be the result of an expression like: fn_list[fn_idx];
    let tmp_arg0 = a;
    let tmp_arg1 = foo(b);

    // End of the function, all variables not used in the `become` call are dropped, as would be done after a `return`.
    // Return value of foo() is *not* dropped as it is moved in the become call to y().
    drop(c);
    // `b` is *not* dropped because it is moved due to the call to foo().
    // `a` is *not* dropped as it is used in the become call to y().
    drop(_arg_one);
    drop(_arg_zero);

    // Finally, `become` the called function.
    become function_ptr(tmp_arg0, tmp_arg1);
}
```

If we used `return` instead, the drops would happen after the call:
```rust
fn x(_arg_zero: Box<()>, _arg_one: ()) {
    let a = Box::new(());
    let b = Box::new(());
    let c = Box::new(());
    return y(a, foo(b));
    // Normal drop order:
    // Return value of foo() is *not* dropped as it is moved in the call to y().
    // drop(c);
    // `b` is *not* dropped because it is moved due to the call to foo().
    // `a` is *not* dropped because it is moved to the callee y().
    // drop(_arg_one);
    // drop(_arg_zero);
}
```

This early dropping allows the compiler to avoid many complexities associated with deciding if the stack frame can be
reused. Instead, the heavy lifting is done by the borrow checker, which will produce a lifetime error if references to
local variables are passed to the called function.  This is distinct from `return`, which _does_ allow references to
local variables to be passed.  Indeed, this difference in the handling of local variables is also the main difference
between `return` and `become`.

### Use Case 1: Recursive Algorithm
A simple example is the following algorithm for summing the elements of a slice.  While this would usually be done with iteration in Rust, this example illustrates a simple use of `become`.  Without guaranteed TCE, this example could overflow the stack if TCO is not applied.

```rust
fn sum_slice(data: &[u64], accumulator: u64) -> u64 {
    match data {
        [first, rest @ ..] => become sum_slice(rest, accumulator + first),
        [] => accumulator,
    }
}
```


### Use Case 2: Interpreter
For an interpreter, the usual loop is to get an instruction, match on that instruction to find the corresponding function, **call** that function, and finally return to the loop to get the next instruction. (This is a simplified example.)

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
    let next_instruction = self.read_instr();
    match next_instruction {
        Instruction::Foo => become self.execute_instruction_foo(),
        Instruction::Bar => become self.execute_instruction_bar(),
    }
}
```

### Pitfall 1: Function calls as arguments are not tail call eliminated.
([original example](https://github.com/rust-lang/rfcs/pull/3407#issuecomment-1516477758))

The guarantee of TCE is only provided to the function call that is an argument to `become`,
it is not given to calls that are arguments, see the following example:

```rust
fn add(a: u64, b: u64) -> u64 {
    a + b
}

pub fn calc(a: u64, b: u64) -> u64 {
    if a < b {
        return a
    }

    let n = a - b;
    become add(calc(n, 2), calc(n, 1));
}
```

In this example `become` will guarantee TCE only for the call to `add()` but not for the `calc()` calls.
Running this code will likely end up in a stack overflow as the recursive calls are to `calc()` which are not TCE'd.

### Pitfall 2: Omission of the `become` keyword causes the call to be `return` instead.
([original example](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-278988088))

This is a potential source of confusion, indeed in a functional language where calls are expected to be TCE this would be quite unexpected. (Maybe in functions that use `become` a lint should be applied that enforces usage of either `return` or `become` in functions where at least one `become` is used.)

```rust
fn foo(x: i32) -> i32 {
    if x % 2 {
        let x = x / 2;
        // one branch uses `become`
        become foo(x);
    } else {
        let x = x + 3;
        // the other does not
        foo(x) // == return foo(x);
    }
}
```

### Pitfall 3: Alternating `become` and `return` calls still grows the stack.
([original example](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-279062656))

Here one function uses `become` the other `return`, this is another potential source of confusion. This mutual recursion
would eventually overflow the stack. As mutual recursion can also happen across more functions, `become` needs to be
used consistently in all functions if TCE is desired. (Maybe it is also possible to create a lint for these
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
Implementation of this feature requires checks that all prerequisites to guarantee TCE are fulfilled.
These checks are:

- The `become` keyword is only used in place of `return`. The intent is to reuse the semantics of a `return` signifying "the end of a function". See the section on [tail-call-elimination](#tail-call-elimination) for examples.
- The argument to `become` is a function (or method) call, that exactly matches the function signature and calling convention of the callee. The intent is to ensure a matching ABI. Note that lifetimes may differ as long as they pass borrow checking, see [below](#return-type-coercion) for specifics on the return type.
- The stack frame of the calling function is reused, this also implies that the function is never returned to. The required checks to ensure this is possible are: no borrows of local variables are passed to the called function (passing local variables by copy/move is ok since that doesn't require the local variable to continue existing after the call), and no further cleanup is necessary. These checks can be done by using the borrow checker as already described in the [section](#difference) showing the difference between `return` and `become` above.
- The restrictions, caused by interactions with other features, are followed. See below for details, the restrictions mostly concern caller context and callee signatures.

If any of these checks fail a compiler error is issued. It is also suggested to ensure that the invariants provided by the pre-requisites are maintained during compilation, raising an ICE if this is not the case.

One additional check must be done, if the backend cannot guarantee that TCE will be performed an ICE is issued. To be specific the backend is required that: "A tail call will not cause unbounded stack growth if it is part of a recursive cycle in the call graph".

The type of the expression `become <call>` is `!` (the never type, see [here](https://doc.rust-lang.org/std/primitive.never.html)). This is consistent with other control flow constructs such as `return`, which also have the type of `!`.

Note that as `become` is a keyword reserved for exactly the use-case described in this RFC there is no backward-compatibility break. This RFC only specifies the use of `become` inside of functions and instead leaves usage outside of functions unspecfied for use by other features.

This feature will have interactions with other features that depend on stack frames, for example, debugging and backtraces. See [drawbacks](#drawbacks) for further discussion.

See below for specifics on interactions with other features.

## Coercions of the Tail Called Function's Return Type
[return-type-coercion]: #return-type-coercion

All coercions that do any work (like deref coercion, unsize coercion, etc) are prohibited.
Lifetime-shortening coercions (`&'static T` -> `&'a T`) are allowed but will be checked by the borrow checker.

Reference/pointer coercions of the return type are **not** supported to minimize implementation effort. Though, coercions which don't change the pointee (`&mut T -> &T`, `*mut T -> *const T`, `&T -> *const T`, `&mut T -> *mut T`) could be added in the future.

Never-to-any coercions (`! -> T`) of the return type are **not** supported to minimize implementation effort. They are difficult to implement and require backend support. To be clear, this only concerns functions that have the never return type like the following example:

```rust
fn never() -> ! {
    loop {}
}

fn tail_call_never_type() -> usize {
    become never(); //~ error: mismatched types
}
```

## Closures
[closures]: #closures

Tail calling closures _and_ tail calling _from_ closures is **not** allowed.
This is due to the high implementation effort, see below, this restriction can be lifted by a future RFC.

Closures use the `rust-call` unstable calling convention, which would need to be adapted to guarantee TCE.
Additionally, any closure that has captures would need special handling, since the captures would currently be dropped before the tail call.

## Variadic functions using `c_variadic`

Tail calling [variadic functions](https://doc.rust-lang.org/beta/unstable-book/language-features/c-variadic.html) _and_ tail calling _from_ variadic functions is **not** allowed.
As support for variadic function is stabilized on a per target level, support for tail-calls regarding variadic functions would need to follow a similar approach. To avoid this complexity and to minimize implementation effort for backends, this interaction is currently not allowed but support can be added with a future RFC.

## Generators

Tail calling from [generators](https://doc.rust-lang.org/beta/unstable-book/language-features/generators.html) is **not** allowed. As the generator state is stored internally, tail calling from the generator function would require additional support to function correctly. To limit implementation effort this is not supported but can be supported by a future RFC.

## Async
[async]: #async

Tail calling _from_ async functions or async blocks is **not** allowed. This is due to the high implementation effort as it requires special handling for the async state machine. This restriction can be relaxed by a future RFC.

Using `become` on a `.await` expression, such as `become f().await`, is also **not** allowed. This is because `become` requires a function call and `.await` is not a function call, but is a special construct.

Note that tail calling async functions from sync code is possible but the return type for async functions is `impl Future`, which is unlikely to be interesting.

## Operators are not supported

Invocations of operators were considered as valid targets but were rejected on grounds of being too error-prone.
In any case, these can still be called as methods. One example of their error-prone nature ([source](https://github.com/rust-lang/rfcs/pull/3407#discussion_r1167112296)):
```rust
pub fn fibonacci(n: u64) -> u64 {
    if n < 2 {
        return n
    }
    become fibonacci(n - 2) + fibonacci(n - 1)
}
```
In this case, a naive author might assume that this is going to be a stack space-efficient implementation since it uses tail recursion instead of normal recursion. However, the outcome is more or less the same since the critical recursive calls are not actually in tail call position.

Further confusion could result from the same-signature restriction where the Rust compiler raises an error since fibonacci and `<u64 as Add>::add` do not share a common signature.

# Drawbacks
[drawbacks]: #drawbacks
<!-- Why should we *not* do this? -->

## Does Not Support General Tail Calls

This proposal is limited to exactly matching function signatures which will *not* allow general tail-calls, however, the work towards this initial version is likely to be useful for a more comprehensive version.

## Implementation Effort and Backend Support

As this feature should be mostly independent of other features the main drawback lies in the implementation and
maintenance effort. This feature adds a new keyword which will need to be implemented not only in Rust but also in other
tooling. However, the primary effort is in correctly interacting with the backends, some of which might not support guaranteeing a tail call (nor guarantee to fail if one is not possible).
Though, support for guaranteed tail calls is improving in common backends:

- LLVM supports a `musttail` marker to indicate that TCE should be performed [docs](https://llvm.org/docs/LangRef.html#id327). Clang which already depends on this feature seems to only generate correct code for the x86 backend [source](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1490009983) (as of 2023-03-30).
- [GCC supports](https://gcc.gnu.org/onlinedocs/gcc/Statement-Attributes.html#index-musttail-statement-attribute) an (mostly) equivalent `musttail` marker.
- WebAssembly also supports guaranteed tail calls as a [standardized feature](https://webassembly.org/features/#table-row-tailcall). However, `tailcall` is currently (July 2025) [not enabled by default](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html#enabled-webassembly-features) for `wasm32-unknown-unknown`.

## Lost Debug Information

There is an unwanted interaction between TCE and debugging. As TCE by design elides stack frames this information is lost during debugging, that is the parent functions and their local variable values are incomplete. As TCE provides a semantic guarantee of constant stack usage it is also not generally possible to disable TCE for debugging builds as then the stack could overflow. (Still, maybe a compiler flag could be provided to temporarily disable TCE for debugging builds. As suggested [here](https://github.com/rust-lang/rfcs/pull/3407/files#r1159817279), another option would be special support for `become` by a debugger. With this support the debugger would keep track of the N most recent calls providing at least some context to the bug.)

## Requires Special Handling of the `track_caller` Attribute

Since `#[track_caller]` adds an argument, the caller location, `#[track_caller]` functions can in theory only tail call other `#[track_caller]` functions.
However, adding or removing `#[track_caller]` is [guaranteed to not cause a breaking change](https://github.com/rust-lang/rust/issues/88302#issuecomment-910614687).
This creates a conflict as adding and removing would cause a breaking change for tail calling functions.

Since the whole goal of this RFC is to support tail calls with the restriction of matching function signatures, there are two options.
Either disallow `#[track_caller]` and tail calling in one function or alternatively build custom support.
Adding custom support will be more feasible when the limitation on exactly matching function signatures can be lifted. 
Thus, since this RFC targets a minimal initial extendable support for guaranteed tail calls, the first option seems appropriate.

Still we need to support interaction with normal functions (non tail calling functions).
Luckily, support for a tail calling function to call a normal function with `#[track_caller]` is easy as it is just a normal call.
However, a more complicated case exists. Tail calling a normal `#[track_caller]` function, from a non `#[track_caller]` function.
As the caller needs to have a matching ABI with the callee, as it is a tail call, this requires a special case adding the location argument without changing the ABI.

A similar issue exists for `#[track_caller]` functions that are [coerced to function pointers](https://doc.rust-lang.org/reference/attributes/codegen.html#r-attributes.codegen.track_caller.decay).
Their support requires appending an implicit parameter to the function ABI, which would be unsound for an indirect call as the location parameter is not part of the function's type.
To resolve this issue a shim is used to sidestep actually passing the extra argument and instead supplying the attributed function’s definition site.
This shim approach seems to be a reasonable choice for this issue as well.

[Alternatively](https://github.com/rust-lang/rust/pull/144762#issuecomment-3146404568),
a calling convention needs to be used
that either ensures all arguments stay in registers, or use a callee-pop convention.
The first being unrealistically restrictive while the second is not supported by backends.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

In this section, the reason for choosing the design is discussed as well as possible alternatives that have been considered.

## Why is this design the best in the space of possible designs?

Of all possible alternatives, this design best fits the tradeoff between implementation effort and functionality while also offering a starting point toward further exploration of a more general implementation. Regarding implementation effort, this design requires the least of backends while not already implementable via a library, see [here](#backend-requirements). Regarding functionality, the proposed design requires function signatures to match, however, this restriction still allows tail calls between functions that _use_ different arguments. This can be done by requiring the programmer to add (unused) arguments to both function definitions so that they match. Additionally, the chosen design allows tail calls for dynamic calls and other variations.

### Creating a Function Local Scope

Using the `become` keyword creates a function local scope that drops all variables not used in the tail call, as would be done for `return`. There is no alternative to this approach as the stack frame needs to be prepared for the tail call.

### Backend Requirements

The main hurdle to implementing this feature is the required work to be done by the backends (e.g. LLVM). See the following list for an overview of approaches that have been considered for this RFC, going by increasing demand on the backends:

1. **Internal Transformation** - Use a MIR transformation to implement tail calls without any backend requirements, possible implementations are: Defunctionalization and [Trampolines](#trampoline-based-approach). However, all proposed options can only support static function calls. This is one reason this option is not chosen, as dynamic function calls seem too important to ignore, see [here](#what-should-be-tail-callable). Another reason is that this approach can already be done by libraries.
2. **Matching Function Signatures** (this RFC) - Require that the caller and callee have matching function signatures. With this restriction, it is possible to do tail calls regardless of the calling convention used, see the next point for why this is important. Though the calling convention needs to match between caller and callee. All that needs to be done by the backends is to overwrite the arguments in place with the values for the tail call. By requiring a matching calling convention and function signature between caller and callee the ABI is guaranteed to match as well. This is also quite similar to how `musttail` is used in practice for Clang: "Implementor experience with Clang shows that the ABI of the caller and callee must be identical for the feature to work [...]" (see [here](#clang)).
3. **Mark Function Definition** - One hurdle to guaranteeing tail calls is that the default calling conventions used by backends usually do not support tail calls and instead another calling convention needs to be used. As it is unreasonable to expect changing the default calling convention, one [option](#attribute-on-function-declaration) is to mark functions that should use a calling convention amenable to tail calls. This requires that backends can support tail calls when allowed to change the calling convention. This requirement, however, already seems quite difficult to establish. For example, this [thread](https://github.com/rust-lang/rfcs/pull/3407#discussion_r1186003262) discusses why this approach is not reasonable.
4. **Backend Specific** - Depend on the backend to decide if a tail call can be performed. While this approach allows gradual advancements it also seems the most unstable and difficult to use.

As described by the list of approaches above, this RFC specifies the approach that is most attainable and still useful in practice while not already implementable via a library.

### What should be Tail Callable

Tail calls can be implemented without backend support if only static calls are supported, see the following list for reasons why other calls should be supported:

- **Dynamic Calls** One example that depends on dynamic tail calls is a C implementation of a Protobuf parser, see [here](https://github.com/rust-lang/rfcs/pull/3407#issuecomment-1500291721).
- **Calls across Crates** This will allow tail calls to library functions, enabling libraries to support code that requires constant stack usage or make calls more performant.
- **Calls to Dynamically Loaded Functions** As an example this would be useful to improve performance for an emulator that uses a JIT.

## What other designs have been considered and what is the rationale for not choosing them?

There are some designs that either can not achieve the same performance or functionality as the chosen approach. Though most other designs evolve around how to mark what should be a tail-call or marking what functions can be tail called. There is also the possibility of providing support for a custom backend (e.g. LLVM) or MIR pass.

### Rust Built-in Functionality

For simple tail recursion on an iterable, [`successors`](https://doc.rust-lang.org/stable/core/iter/fn.successors.html) can be used to compute a result for each element.
Nearly equivalently, a (combinator)[https://users.rust-lang.org/t/when-will-rust-have-tco-tce/20790/3] can be used to express a tail recursive function, however, allowing more flexibility regarding the returned result.

These approaches do not provide a way to express general tail calls, so do not fulfill a basic requirement we would like to achieve.

### Trampoline based Approach
There could be a trampoline-based approach
([comment](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-326952763)) that can fulfill the semantic guarantee
of using constant stack space, though they can not be used to achieve the performance that the chosen design is capable
of.

Similarly, as mentioned [here](https://github.com/rust-lang/rfcs/pull/3407#discussion_r1190464739), an approach used by Chicken Scheme is to do normal calls and handle stack overflows by cleaning up the stack.

### Principled Local Goto
One partial alternative would be to support some kind of local goto, indeed there already exists work in this direction: [pre-RFC](https://internals.rust-lang.org/t/pre-rfc-safe-goto-with-value/14470/9?u=scottmcm) ([comment](https://github.com/rust-lang/rfcs/issues/2691#issuecomment-1458604986)) or another simililar [approach](https://internals.rust-lang.org/t/idea-for-safe-computed-goto-using-enums/21787?u=programmerjake). These designs should be able to achieve the targeted performance characteristics. Also, similar to this RFC, these approaches require backend support for computed gotos (that is jumping to an address in a variable not just a label), instead of guaranteed tail calls. However, crucially, a local goto does not replace tail calls, as it is purely a function local optimization. So while this alternative could be used in some situations instead of guaranteed tail calls they fundamentally cannot replace each other.

### Attribute on Function Declaration
[attribute-on-function-declaration]: #attribute-on-function-declaration

One alternative is to mark a group of functions that should be mutually tail-callable [example](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-1161525527) with some follow-up [discussion](https://github.com/rust-lang/rfcs/pull/1888#issuecomment-1185828948).

The goal behind this design is to allow TCE of functions that do not have exactly matching function signatures, in
theory, this just requires that tail-called functions are callee cleanup, which is a mismatch to the default calling
convention used by Rust. To limit the impact of this change all functions that should be TCE-able should be marked with
an attribute.

While quite noisy it is also less flexible than the chosen approach. Indeed, TCE is a property of the call and not a
function definition, sometimes a call should be guaranteed to be TCE, and sometimes not, marking a function would
be less flexible.

### Adding a mark to `return`

The return keyword could be marked using an attribute or an extra keyword as in the example below.

```rust
fn a() {
    // The chosen variant.
    become b();

    // Using an attribute.
    #[become]
    return b();

    // Adding an extra keyword.
    return become b();
}
```

These alternatives mostly come down to personal taste (or bikeshedding) and the plain keyword `become` was chosen because of the following reasons:

- It is [reserved](https://rust-lang.github.io/rfcs/0601-replace-be-with-become.html) exactly this use case.
- It is shorter to write.
- The behavior changes in subtle ways compared to a plain `return`. To clearly indicate this change in behavior a stronger distinction from `return` than adding a mark seems warranted.
  - TCE as proposed in this RFC requires dropping local variables before the function call instead of after with `return`.
  - From a type system perspective the type of the `return` expression (`!`, the never type, see [here](https://doc.rust-lang.org/std/primitive.never.html) for an example) stays the same even when adding one of the markings. This means that type-checking can not help if the marking is forgotten or added mistakenly. (Note that the argument, the function call, can still be type checked, just not the `return` expression.)

### Require Explicit Dropping of Variables

(Based on this [comment](https://github.com/rust-lang/rfcs/pull/3407#issuecomment-1532841475))

An alternative approach could be to refuse to compile functions that would need to run destructors before `become`.
This would force code to not rely on implicit drops and require calls to `drop(variable)`, as in the following example:

```rust
fn f(x: String) {
    drop(x); // necessary
    become g();
}
```

This approach would result in more verbose code but would also be easier to read for people not familiar with tail calls.
Also, it is forwards compatible with implicit dropping before `become`.

The reason this approach is not chosen is that the tradeoff between increased verbosity and the reduction of initial learning time seems to not be worth it.
Additionally, implementing the diagnostic for forgotten drops can be expected to be more effort than for correct drop elaboration.

### Custom Compiler or MIR Passes
One more distant alternative would be to support a custom compiler or MIR pass so that this optimization can be done externally. While supported for LLVM [Zulip](https://rust-lang.zulipchat.com/#narrow/stream/187780-t-compiler.2Fwg-llvm/topic/.E2.9C.94.20Running.20Custom.20LLVM.20Pass/near/320275483), for MIR this is not supported [discussion](https://internals.rust-lang.org/t/mir-compiler-plugins-for-custom-mir-passes/3166/10).

This would be an error-prone and unergonomic approach to solving this problem.


## What is the impact of not doing this?
> Rust's goal is to empower everyone to build reliable and efficient software.
([source](https://blog.rust-lang.org/inside-rust/2022/04/04/lang-roadmap-2024.html))

This feature provides a crucial optimization for some low-level code. It seems that without this feature there is a big
incentive for developers of those specific applications to use other system-level languages that can guarantee TCE.

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
feature (TCE for general calls). For system-level languages, TCE is usually wanted but implementation
effort is a common reason this is not yet done. Even languages with managed code such as .Net or ECMAScript (as per the
standard) also support TCE, again performance and resource usage were the main motivators for their
implementation.

See below for a more detailed description of select compilers and languages.


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
As of 2024, GCC supports a [mostly LLVM compatible](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=83324#c27) version of `musttail`, documented here: https://gcc.gnu.org/onlinedocs/gcc/Statement-Attributes.html#index-musttail-statement-attribute.

> The gnu::musttail or clang::musttail standard attribute or musttail GNU attribute can be applied to a return statement with a return-value expression that is a function call. It asserts that the call must be a tail call that does not allocate extra stack space, so it is safe to use tail recursion to implement long-running loops.
> 
> `[[gnu::musttail]] return foo();`
> 
> `__attribute__((musttail)) return bar();`
> 
> If the compiler cannot generate a musttail tail call it reports an error. On some targets, tail calls may not be supported at all. The musttail attribute asserts that the lifetime of automatic variables, function parameters and temporaries (unless they have non-trivial destruction) can end before the actual call instruction, and that any access to those from inside of the called function results is considered undefined behavior. Enabling -O1 or -O2 can improve the success of tail calls. 


## WebAssembly
The [proposal](https://github.com/WebAssembly/tail-call/blob/master/proposals/tail-call/Overview.md) for tail calls in WebAssembly has been accepted and has been [implemented](https://webassembly.org/features/#table-row-tailcall) by all major browsers and the Wasmtime runtime (as of 11-07-2025).

## Zig
Zig provides separate syntax to allow more flexibility than normal function calls. There are options for async calls, inlining, compile-time evaluation of the called function, or specifying TCE on the call.
([source](https://ziglang.org/documentation/master/#call))

The following is an example taken from here (https://zig.godbolt.org/z/v13vrjxG4, a toy lexer using tail calls in Zig):

```zig
export fn lex(data: *Data) callconv(.C) u32
{
    if(data.cursor >= data.input.len)
        return data.tokens;
    switch(data.input[data.cursor]) {
        'a' => return @call(.always_tail, lex_a, .{data}),
        'b' => return @call(.always_tail, lex_b, .{data}),
        else => return @call(.always_tail, lex_err, .{data}),
    }
}
```

## Carbon
As per this [issue](https://github.com/carbon-language/carbon-lang/issues/1761) it seems providing TCE is of interest even if the implementation is difficult.


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
    - One point that needs to be decided is if TCE should be a feature that needs to be required from all backends or if it can be optional. Currently, the RFC specifies that an ICE should be issued if a backend cannot guarantee that TCE will be performed.
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
    - Are all calling-convention used by Rust available for TCE with the proposed restrictions on function signatures?
    - Is there some way to reduce the impact on debugging and other features?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of
  the solution that comes out of this RFC?
  - Supporting general tail calls, the current RFC restricts function signatures which can be loosened independently in the future.

## Resolved Questions

- Can generic functions be supported?
  - As Rust uses Monomorphization, generic functions are not a problem.
- Can dynamic function calls be supported?
  - Dynamic function calls are supported ([confirmation](https://github.com/rust-lang/rfcs/pull/3407#discussion_r1191600480)).
- Can functions outside the current crate be supported, functions from dynamically loaded libraries?
  - Same as dynamic function calls these function calls are supported ([confirmed for LLVM](https://github.com/rust-lang/rfcs/pull/3407#discussion_r1191602364)).
- Can closures be supported?
  - Closures are **not** supported see [here](#closures).
- Can async functions be supported?
  - Async functions are **not** supported see [here](#async).
- Should "performance" be guaranteed by the backends?
  - "Performance" is **not** guaranteed by the backends, see [here](#performance-guarantee).

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

## Lints

The functionality introduced by RFC also has possible pitfalls, it is likely worthwhile to provide lints that warn of these issues. See the discussion [here](https://github.com/rust-lang/rfcs/pull/3407#discussion_r1159822824) for possible lints.

Additionally, there can be another class of lints, those that guide migration to using `become`.
For example, provide a lint that indicates if a trivial transformation from `return` to `become` can be done for function calls where all requisites are already fulfilled. Note that, this lint might be confusing and noisy.

## Helpers
[helpers]: #helpers

It seems possible to keep the restriction on exactly matching function signatures by offering some kind of placeholder
arguments to pad out the differences. For example:

```rust
fn foo(a: u32, b: u32) {
    // uses `a` and `b`
}

fn bar(a: u32, _b: u32) {
    // only uses `a`
}
```

Maybe it is useful to provide a macro or attribute that inserts missing arguments.

```rust
#[pad_args(foo)]
fn bar(a: u32) {
    // ...
}
```

## Relaxing the Requirement of Strictly Matching Function Signatures for Static Calls

It should be possible to automatically pad the arguments of static tail calls, similar to the [helpers section](#helpers) above. See this [comment](https://github.com/rust-lang/rfcs/pull/3407#issuecomment-1500620309) for details. Note that this approach does not relax requirements for dynamic calls.

## Relaxing the Requirement of Strictly Matching Function Signatures with a new Calling Convention

In the future, a calling convention could be added to allow `become` to be used with functions that have mismatched function signatures. This approach is close to the alternative of [adding a marker to the function declaration](#attribute-on-function-declaration). Same as the alternative, a requirement needs to be added that backends provide a calling convention that support tail calling.

## Mismatches in Mutability

Mismatches in mutability (like `&T` <-> `&mut T`) for arguments and return type of the function signatures are currently not supported as they are different types. However, this mismatch could be supported if there is a guarantee that mutability has no effect on ABI. For more details, see [here](https://github.com/rust-lang/rfcs/pull/3407#discussion_r1193897615).

## Performance Guarantee

First of all, performance is ambiguous. As a stand in, we could instead require that no new stack frame is created for a tail call. The reason for this choice is that creating a new stack frame can be the cause slowdowns in hot loops that do many calls, which is a code pattern that can likely be optimized with tail calls.

Can the requirement to not create new stack frames when using tail calls be imposed on backends? This answer seems to be no, even for LLVM. LLVM provides some [guarantees](https://llvm.org/docs/LangRef.html#call-instruction) for tail calls, however, none do ensure that no new stack frame is created (as of 24-05-2023).

If it turns out that in practice the "no new stack frame requirement" is not guaranteed by backends it might be worthwhile to revisit this performance requirement.

## Functional Programming

This might be wishful thinking but if TCE is supported there could be further language extensions to make Rust
more attractive for functional programming paradigms. Though it is unclear to me how far this should be taken or what
changes exactly would be a benefit.
