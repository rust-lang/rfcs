- Feature Name: `loop_match`
- Start Date: 2024-09-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3720)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC adds `loop match`:

- a `loop` and `match` can be fused into a `loop match <scrutinee> { /* ... */ }`
- a `loop match` can be targeted by a `continue <value>`. The `value` is treated as a replacement operand to the `match` expression.

The state transitions (going from one branch of the match to another) can be annotated with the `const` keyword, providing more accurate CFG information to the backend. That means:

- more valid programs are accepted, increasing expressivity
- the backend can generate better code, leading to significant speedups

### Basic example

A `loop match` is similar to a `match` inside of a `loop`, with a mutable variable being updated to move to the next state. For instance, these two functions are semantically equivalent:

```rust
fn loop_match() -> Option<u8> {
    loop match 1u8 {
        1 => continue 2,
        2 => continue 3,
        3 => break Some(42),
        _ => None
    }
}

fn loop_plus_match() -> Option<u8> {
    let mut state = 1u8;
    loop {
        match state {
            1 => { state = 2; continue; }
            2 => { state = 3; continue; }
            3 => { break Some(42) }
            _ => { break None }
        }
    }
}
```

### Interesting example

The real power of `loop match` lies in giving the compiler more accurate information about the control flow of a program. Consider

```rust
enum State { Foo, Bar, Baz, }

let owned = Box::new(1);
let mut state = State::Foo;
loop {
    match state {
        State::Foo => state = State::Bar,
        State::Bar => {
            // or any function that moves the value
            drop(owned); // ERROR use of moved value: `owned`
            state = State::Baz;
        }
        State::Baz => break,
    }
}
```

Reading the code, it is obvious that state moves from states `Foo` to `Bar` to `Baz`: no other path is possible. Specifically, we cannot end up in `State::Bar` twice, and hence the generated "use of moved value" error is not a problem in practice. This program is valid, but nonetheless rejected by the rust compiler.

With `loop const match` and `const continue` the compiler now understands the control flow:

```rust
loop const match State::Foo {
    State::Foo => const continue State::Bar,
    State::Bar => {
        // or any function that moves the value
        drop(owned); // all good now!
        const continue State::Baz;
    }
    State::Baz => break,
}
```

The following sections go into why this feature is essential for writing efficient state machines, looking both at ergonomics and performance. See [the implementation notes](#Implementation-notes) for instructions on running our proof-of-concept implementation.

# Motivation
[motivation]: #motivation

The goal of `loop match` is improved ergonomics and codegen for state machines. Rust, being a systems language, should be good at writing efficient state machines, and currently falls short. Complex state machines are niche, but foundational to many programs (parsers, interpreters, networking protocols).

This RFC follows in part from work on [zlib-rs](https://github.com/trifectatechfoundation/zlib-rs) and [libbzip2-rs](https://github.com/trifectatechfoundation/libbzip2-rs). The decompression functions of zlib and bzip2 contain a large state machine. The C versions rely heavily on:

- putting values onto the stack (rather than behind a heap-allocated pointer). In practice, LLVM is a lot better at reasoning about stack values, resulting in a smaller stack and better optimizations
- guaranteed direct jumps between states, using the fallthrough behavior of C `switch` statements

Today, we simply cannot achieve the same codegen as C implementations. This limitation actively harms the adoption of rust in performance-sensitive domains like compression.

## Ergonomics

State machines require flexible control flow. However, the unstructured control flow of C is in many ways too flexible: it is hard for programmers to follow and for tools to reason about and give good errors for. Ideally, there is a middle ground between code that is easy to understand (by human and machine), interacts well with other rust features, and is flexible enough to efficiently express state machine logic.

Additonally rust is a lot more strict than C: values must be initialized before use, and cannot be used after they have been dropped or moved. The analysis to determine whether a value can be used is conservative: there are valid programs (that would not exhibit incorrect behavior at runtime) that are nonetheless rejected by the rust compiler. Accepting more valid programs while still rejecting all incorrect programs is an improvement.

Today there is no good way to translate C code that uses implicit fallthroughs or similar control flow to rust while preserving both the ergonomics (in particular, a consistent level of indentation) and the performance (due to LLVM using jump tables instead of an unconditional jump, see the next section). If we wanted to translate this C code to Rust:

```c
switch (a) {
    case 1:
        i += 1;
        /* implicit fallthrough */
    case 2:
        i += 1;
        break;
    default:
}
```

We could try a solution with nested labeled blocks, but it scales very poorly in the number of states:

```rust
'done: {
    'case_2: {
        'case_1: {
            match a {
                1 => break 'case_1,
                2 => break 'case_2,
                _ => break 'done,
            }
        }

        i += 1;
        /* implicit fallthrough */
    }

    i += 1;
    break 'done;
};
```

This does not spark joy. Macros [have been proposed](https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang/topic/Fallthrough.20in.20Match.20Statements/near/472962729) to tame the explosion of levels of indentation, but that just introduces custom syntax to learn for something as fundamental to a low level programming language as a state machine. Furthermore, editor experience within macros is still not as good as for first-class language constructs.

Alternatively, we could try to introduce a loop (we'll refer to this as the "loop + match" approach):

```rust
let mut a = a;
loop {
    match a {
        1 => {
            i += 1;
            a = 2;
            continue;
        }
        2 => {
            i += 1;
            break;
        }
        _ => break,
    }
}
```

This keeps indentation flat, and it is much easier to understand the control flow. But (in general) this loop version is less efficient than the original C code, because the transition between states is not always a direct jump, even if the compiler in theory could know exactly what the next block of code to execute is (again, see the next section for details).

A `loop match` solves both the ergonomics issue and makes reliably generating efficient code much easier:

```rust
loop match a {
    1 => {
        i += 1;
        const continue 2;
    }
    2 => {
        i += 1;
        break;
    }
    _ => break,
}

// or even

loop match a {
    1 => {
        i += 1;
        const continue 2;
    }
    2 => i += 1,
    _ => {}
}
```

One could argue that the inability to directly translate switch fallthrough into rust is an instance of the [XY problem](https://xyproblem.info/), but many parsers, interpreters and other state machines just rely on this kind of control flow.

A niche, but very valuable use case is [c2rust](https://github.com/immunant/c2rust), a tool that automatically translates C to rust. In many cases, a C `switch` cannot be automatically translated to a rust `match` due to implicit fallthroughs, so the translation produces an abomination of labeled blocks and loops: semantically correct, but hard to reason about. Currently, such cases have to be [cleaned up by hand](https://github.com/trifectatechfoundation/libbzip2-rs/pull/25), which is error-prone. Being able to lower such control flow, in most cases, to a `loop match` greatly improves both the readability of the generated code, speeding up the porting process.

Many other parser, decoder and other lowlevel crates will similarly benefit from the ergonomics of `loop match`.

## Code generation

State machines (parsers, interpreters, ect) can be written as a loop containing a match on the current state. The match picks the branch that belongs to the current state, some logic is performed, the state is updated, and eventually control flow jumps back to the top of the loop, branching to the next state.

```rust
loop {
    match state {
        A => {
            // <perform work>

            state = B;
        }
        B => {
            // ...
        }
        // ...
    }
}
```

While this is a natural way to express a state machine, it is well-known that when translated to machine code in a straightforward way, this approach is inefficient on modern CPUs:

- The match is an unpredictable branch, causing many branch misses. Reducing the number of branch misses is crucial for good performance on modern hardware.
- The "loop + match" approach contains control flow paths (so, sequences of branches) that will never be taken in practice. More opimizations are possible if the actual possible paths are known more precicely (e.g. stack space can be reused if the value stored there will not be used in later states).

By providing the compiler with more precise knowlege about what state transitions actually exists (i.e. what other states can follow a particular state), we get major performance improvements in practice. A proof of concept implementation of `loop match` shows considerable performance gains versus current recommended workarounds in real-world scenarios ([all results](https://gist.github.com/folkertdev/977183fb706b7693863bd7f358578292)):

```
Benchmark 3 (80 runs): /tmp/labeled-match-len rs-chunked 4 silesia-small.tar.gz
  measurement          mean ± σ            min … max           outliers         delta
  wall_time          62.6ms ±  555us    61.7ms … 66.1ms          2 ( 3%)        ⚡- 14.0% ±  1.4%
  peak_rss           24.1MB ± 77.9KB    23.9MB … 24.1MB          0 ( 0%)          -  0.1% ±  0.1%
  cpu_cycles          249M  ± 1.87M      248M  …  263M           5 ( 6%)        ⚡- 15.4% ±  1.3%
  instructions        686M  ±  267       686M  …  686M           0 ( 0%)        ⚡- 24.9% ±  0.0%
```

So clearly, better code generation is possible, and not reliably achieved today.

## Doesn't LLVM optimize this already?

No.

In some cases, the LLVM backend already achieves this optimal code generation using unconditional jumps, but the transformation is not guaranteed and fails for more complex inputs. Furthermore, LLVM is not the only rust codegen backend: it is likely that both `rustc_codegen_gcc` and `rustc_codegen_cranelift` will see more and more use. Hence we should be sceptical of relying on LLVM to achieve good codegen, and prefer performing optimization for all backends on the rustc MIR representation.

Nevertheless, we can use LLVM as a reference point for what will already get optimized today, and where code generation is lacking.

**targets are statically known**

In this example all jump targets are statically known, and LLVM gives us the desired unconditional jumps between the states ([godbolt link](https://godbolt.org/z/x9aePGxWT)):

```rust
#[allow(dead_code)]
enum State { S1, S2, S3 }

#[no_mangle]
#[rustfmt::skip]
unsafe fn looper(mut state: State, input: &[u8]) {
    for from in input {
        match state {
            State::S1 => {
                print("S1");
                match *from {
                    0 => return,
                    _ => state = State::S2,
                }
            }
            State::S2 => {
                print("S2");
                match *from {
                    0 => return,
                    _ => state = State::S3,
                }
            }
             State::S3 => {
                print("S3");
                match *from {
                    0 => return,
                    _ => state = State::S1,
                }
            }
        }
    }
}

extern "Rust" {
    fn print(s: &str);
}
```

**targets are dynamically known**

When the jump targets are only known at runtime, LLVM generates a jump table, the best it can do ([godbolt link](https://godbolt.org/z/d39oaKG4P)):

```rust
unsafe fn looper(mut state: State, input: &[u8]) {
    let mut i = 0;
    loop {
        match state {
            State::S1 => { state = process_1(*input.get_unchecked(i)); i += 1; continue; }
            State::S2 => { state = process_2(*input.get_unchecked(i)); i += 1; continue; }
            State::S3 => { state = process_3(*input.get_unchecked(i)); i += 1; continue; }
            State::S4 => { state = process_4(*input.get_unchecked(i)); i += 1; continue; }
        }
    }
}
```

The generated jump table and jumping logic looks like this. In particular, the jump is now to a register `jmp rax` instead of to a label `jmp .LBB0_6`. Jump tables (also known as computed goto) are better than the naive "jump to the top of the loop, then switch on the state" approach, but worse than unconditional branches.

```asm
        lea     r15, [rip + .LJTI0_0]
        movsxd  rax, dword ptr [r15 + 4*rax]
        add     rax, r15
        jmp     rax

.LJTI0_0:
        .long   .LBB0_1-.LJTI0_0
        .long   .LBB0_2-.LJTI0_0
        .long   .LBB0_3-.LJTI0_0
        .long   .LBB0_4-.LJTI0_0
```

**suboptimal codegen**

So far LLVM generates (close to) optimal code. But neither rustc nor LLVM guarantee that a jump to a compile-time known target is really turned into a direct jump in assembly. We can confuse the LLVM optimizer by adding more state transitions, making it generate a jump table in a program where it is definitely possible to just use direct jumps. Consider ([godbolt link](https://godbolt.org/z/M81bva87o)):

```rust
#[allow(dead_code)]
enum State {
    Done,
    S1,
    S2,
    S3,
}

#[no_mangle]
#[rustfmt::skip]
unsafe fn looper(input: &[u8]) -> usize {
    let mut state = State::S1;

    let mut it = input.iter();

    loop {
        match state {
            State::S1 => {
                let Some(from) = it.next() else { state = State::Done; continue };

                match from {
                    0 => return 1,
                    _ => state = State::S2
                }
            }
            State::S2 => {
                let Some(from) = it.next() else { state = State::Done; continue };

                match from {
                    0 => return 2,
                    _ => state = State::S3
                }
            }
            State::S3 => {
                let Some(from) = it.next() else { state = State::Done; continue };

                match from {
                    0 => return 3,
                    _ => state = State::S1,
                }
            }
            State::Done => {
                return 0;
            }
        }
    }
}
```

In this example, all state transitions should be clear, and it should be possible to turn each jump into a direct jump. However, LLVM generates the following assembly:

```asm
looper:
        add     rsi, rdi
        mov     eax, 1
        lea     rcx, [rip + .LJTI0_0]
.LBB0_1:
        mov     rdx, rdi
        movsxd  rdi, dword ptr [rcx + 4*rax]
        add     rdi, rcx
        jmp     rdi
.LBB0_5:
        cmp     rdx, rsi
        je      .LBB0_6
        lea     rdi, [rdx + 1]
        mov     eax, 2
        cmp     byte ptr [rdx], 0
        jne     .LBB0_1
        jmp     .LBB0_9
.LBB0_2:
        cmp     rdx, rsi
        je      .LBB0_6
        lea     rdi, [rdx + 1]
        mov     eax, 3
        cmp     byte ptr [rdx], 0
        jne     .LBB0_1
        jmp     .LBB0_4
.LBB0_10:
        test    rdx, rdx
        setne   r8b
        xor     r9d, r9d
        cmp     rdx, rsi
        setne   r9b
        lea     rdi, [r9 + rdx]
        mov     eax, 0
        test    r8b, r9b
        je      .LBB0_1
        cmp     byte ptr [rdx], 0
        mov     eax, 1
        jne     .LBB0_1
        mov     eax, 3
        ret
.LBB0_6:
        xor     eax, eax
.LBB0_7:
        ret
.LBB0_4:
        mov     eax, 2
        ret
.LBB0_9:
        mov     eax, 1
        ret
.LJTI0_0:
        .long   .LBB0_7-.LJTI0_0
        .long   .LBB0_5-.LJTI0_0
        .long   .LBB0_2-.LJTI0_0
        .long   .LBB0_10-.LJTI0_0
```

LLVM has generated a jump table, and all state transitions go via this jump table. For the branches, this is done with the `jne .LBB0_1` jump, but even the initial pattern match goes via the jump table where LLVM definitely should know that we're in `State::S1`:

```asm
.LBB0_1:
        mov     rdx, rdi
        movsxd  rdi, dword ptr [rcx + 4*rax]
        add     rdi, rcx
        jmp     rdi
```

This code generation is bad! This example should generate direct jumps to the next state, but even if it didn't, it should duplicate the jump table lookup logic (the `jmp rdi` specifically) to each `match` branch, so that the branch predictor can keep track of each `match` branch individually where it is most likely to jump to next.

As a programmer, we have no control over the code generation. Adding one extra state transition to your program, or making some other small change, can thus cause a major performance regression.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

---

A `loop` and `match` can be combined into a `loop match`. A `loop match` can be the target of a `continue value` expression. The `value` replaces the operand of the `match` for the next iteration of the loop. These functions are equivalent:

```rust
fn loop_match() -> Option<u8> {
    loop match 1u8 {
        1 => continue 2,
        2 => continue 3,
        3 => break Some(42),
        _ => None
    }
}

fn loop_plus_match() -> Option<u8> {
    let mut state = 1u8;
    loop {
        match state {
            1 => { state = 2; continue; }
            2 => { state = 3; continue; }
            3 => { break Some(42) }
            _ => { break None }
        }
    }
}
```

So far `loop match` is just syntax sugar. Its power lies in a combination with the `const` keyword, that provides the compiler with more accurate
information about the control flow of your program.

For example, this program is valid because with `loop const match` and `const continue` all paths that makes it to the `false` branch will have initialized the `x` variable:

```rust
let x: u64;

loop const match true {
    true => {
        x = 42;
        const continue false;
    }
    false => {
        dbg!(x)
    }
}
```

The more precise control flow information is also used by the borrow checker, so that more valid programs are accepted. This program that uses a standard `loop` and `match` and runs into a "use of moved value" error:

```rust
enum State { Foo, Bar, Baz, }

let owned = Box::new(1);
let mut state = State::Foo;
loop {
    match state {
        State::Foo => state = State::Bar,
        State::Bar => {
            // or any function that moves the value
            drop(owned); // ERROR use of moved value: `owned`
            state = State::Baz;
        }
        State::Baz => break,
    }
}
```

Reading the code, it is obvious that state moves from states `Foo` to `Bar` to `Baz`: no other path is possible. Specifically, we cannot end up in `State::Bar` twice, and hence the generated "use of moved value" error is not a problem in practice. This program is valid, but rejected by the rust compiler.

By using `loop match` and annotating the state transitions with `const`, the compiler now understands the control flow:

```rust
loop const match State::Foo {
    State::Foo => const continue State::Bar,
    State::Bar => {
        // or any function that moves the value
        drop(owned); // all good now!
        const continue State::Baz;
    }
    State::Baz => break,
}
```

This more accurate understanding of control flow has advantages for the borrow checker, but also for other downstream compiler passes.

To use a `loop const match <expr>` or `const continue <expr>` expression, the `<expr>` must be [static-promotable](https://github.com/rust-lang/rfcs/pull/1414).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

---

The changes to the language are:

- we add `loop match` expressions: `loop match scrutinee { ... }`
- `continue <operand>` expressions can target the `loop match`, replacing `scrutinee` with `<operand>` and proceeding to the correct match branch
- state transitions can be annotated with the `const` keyword: `loop const match` and `const continue`

The const-annotated state transitions provide more accurate CFG information to the backend:

- such transitions must occur on static-promotable values (see below)
- such transitions are lowered from HIR to MIR as a `goto` to the right `match` branch, and can hence express irreducible control flow

## Restrictions on the `loop const match` and `const continue` operand

This RFC proposes a conservative condition for when a state transition can be marked as `const`: the value must be eligible for "static promotion" as introduced in [RFC 1414](https://github.com/rust-lang/rfcs/blob/master/text/1414-rvalue_static_promotion.md). These are expressions that would compile in the following snippet:

```rust
let x: &'statix _ = &<expr>;
```

For these values, it can always be statically known exactly which branch the value ends up in. This limited support is sufficient for translating C state machines that use `goto` and labels.

Of cousre, compared to the full power of rust patterns, this limitation is unfortunate. Specifically, in this RFC, the `const continue` in the `None` branch here will be rejected:

```rust
use core::hint::black_box;

loop match None {
    None => {
        println!("None");
        const continue Some(black_box(true));
    }
    Some(false) => {
        println!("Some(false)");
        const continue Some(false);
    }
    Some(true) => {
        println!("Some(true)");
        break;
    }
}
```

Intuitively, a `goto` could be inserted to the `Some(_)` pattern (which does not exist in the surface language, but its equivalent is inserted by pattern match desugaring). However, dealing with partial patterns leaks information about the order in which patterns are evaluated. There's an ongoing discussion about whether rust can/should commit to a particular order or not.

Unfortunately, the following snippet is also rejected even though here the desired behavior is clear. We just don't currently have an accurate way of describing that this snippet is valid and the one above is not, so we conservatively reject both.

```rust
loop match None {
    None => {
        println!("None");
        const continue Some(core::hint::black_box(true));
    }
    Some(b) => match b {
        false => {
            println!("Some(false)");
            const continue Some(false);
        }
        true => {
            println!("Some(true)");
            break;
        }
    }
}
```
Expanding the set of expressions that is accepted is therefore left as future work.

## Edge cases

Overall, behavior is consistent with `loop { match scrutinee { ... } }`

### labels

Labels work as expected for `loop`:

```rust
fn loop_match() -> Option<u8> {
    'label: loop match 1u8 {
        1 => continue 'label 2,
        2 => continue 'label 3,
        3 => break 'label Some(42),
        _ => None
    }
}
```

### `continue <operand>` only in `loop match`

The `const? continue <operand>` expression is only allowed within `loop match` expressions. Usage elsewhere errors, analogous to `break` with value from `for` and `while` loops:

```rust
for i in 0..10 {
    continue 42;
}
```

```
error[E0571]: `continue` with value from a `for` loop
 --> src/main.rs:3:9
  |
2 |     for i in 0..10 {
  |     -------------- you can't `continue` with a value in a `for` loop
3 |         continue 53i64;
  |         ^^^^^^^^^^^^^^ can only continue with a value inside `loop match`
  |
help: use `continue` on its own without a value inside this `for` loop
  |
3 |         continue;
  |         ~~~~~~~~
```

### no ambiguity

If unlabeled, `continue <operand>` and `const continue <operand>` continue the innermost loop.

If it is unclear what the user intended when the innermost loop is not a `loop match` (but one of `loop`, `for`, `while`), an error is emitted. We assume that they did in fact intend to continue a `loop match` and emit an error that is analogous to how unlabeled breaks are not allowed in labeled blocks:

```rust
loop match () {
    () => {
        for i in 0..10 {
            continue ();
        }
    }
}
```

```
error[E0000]: `continue` with value from a nested loop
 --> src/main.rs:3:9
  |
4 |         continue ();
  |         ^^^^^^^^^^^ `continue` with value in a nested loop must bear a label
  |
```

### `const continue` must know where to jump

It must be known at compile time which branch the operand of a `loop const match <operand>` or `const continue <operand>` will jump to.
The rules are described [here](#restrictions-on-the-const-continue-operand).
If the value is not of the right form, it will be rejected:

```rust
loop match 1u8 {
    0 => break,
    _ => const continue core::hint::black_box(42),
}
```

```
error[E0000]: `continue` with value target unknown at compile time
 --> src/main.rs:3:9
  |
4 |         const continue core::hint::black_box(42),
  |                        ^^^^^^^^^^^^^^^^^^^^^^^^^ this target of a `continue` is not known at compile time
  |
help: use a non-const `continue` with value instead
  |
4 |         continue core::hint::black_box(42),
  |         ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
```

### plain `continue` in a `loop match`

A plain `continue` jumps back to the top of the loop like a standard `loop { match _ { ... } }`, so this is accepted:

```rust
loop match read_next_byte() {
    0 => continue 1,
    _ => continue,
}
```

### `loop const match` and `const continue` can be freely mixed

We can have a `loop const match` with non-const continues:

```rust
loop const match State::Start {
    State::Start => continue next_state(),
    State::S2 => break,
    State::S3 => const continue State::S2,
    State::Finished => break,
}
```

And likewise `const continue` is valid in a non-const `loop match`. In all cases, we only get the improved MIR lowering for the transitions that are explicitly annotated as `const`.

It might make sense to have clippy lints to enforce that a certain `loop match` should only have `const` transitions.

## Proof of Concept

A proof of concept of an earlier version of this RFC has already been by @bjorn3, to show that the approach is feasible and provides the anticipated runtime improvements. The syntax used in this PoC is now outdated, but the code generation aspects are still relevant.

See the [earlier version of this section](https://github.com/folkertdev/rust-rfcs/blob/1b8a6ba27ed1a92cf8f5573f3e070125b5f579be/text/3720-labeled-match.md) for details on that implementation.

## `HIR -> MIR` Lowering

The meat of this proposal. The core idea is that `loop const match value` and `const continue value` are desugared into a `goto` to the `match` branch that `value` matches.

### Intuition

This snippet

```rust
enum State {A, B }

fn example(state: State) {
    let mut state = state;
    loop {
        match state {
            State::A => {
                // perform work
                state = State::B;
                continue;
            }
            State::B => {
                break 42
            }
        }
    };
}
```

Produces this MIR today with `--release`. Assuming the initial state is `State::A`, the control flow starts in `bb1`, jumps to `bb4` which updates the state, back to `bb1`, then to `bb3`. The `switchInt` is an unpredictable branch which is taken for every state transition.

```
    bb1: {
        _3 = discriminant(_2);
        switchInt(move _3) -> [0: bb4, 1: bb3, otherwise: bb2];
    }

    bb2: {
        unreachable;
    }

    bb3: {
        StorageDead(_2);
        return;
    }

    bb4: {
        _2 = const State::B;
        goto -> bb1;
    }
```

> NOTE: in theory, a MIR analysis pass should be able to simplify the control flow here. However those working on MIR optimizations [appear sceptical](https://rust-lang.zulipchat.com/#narrow/channel/131828-t-compiler/topic/improving.20rust.20codegen.20at.20the.20GOSIM.20unconf/near/478588328) that this would be a good idea.

The proposed `loop match` code

```rust
enum State {A, B }

fn example(state: State) {
    loop match state {
        State::A => {
            // perform work
            const continue State::B;
        }
        State::B => {
            break 42
        }
    };
}
```

will instead generate

```
    bb1: {
        _3 = discriminant(_2);
        switchInt(move _3) -> [0: bb4, 1: bb3, otherwise: bb2];
    }

    bb2: {
        unreachable;
    }

    bb3: {
        StorageDead(_2);
        return;
    }

    bb4: {
        _2 = const State::B;
        goto -> bb3;
    }
```

So that control flow is now starting in `bb1`, via `bb4` directly moving to `bb3`. The `State::A -> State::B` (i.e. `bb4 -> bb3`) transition is a direct jump, and also `bb1` will never jump to `bb3` if the initial input is never `State::B`. The branch predictor should be able to pick up on this pattern too.

### Lowering Details: `continue value`

Semantically this should behave as if `loop match` were desugared to `loop { match _ { ... } }`, especially with regards to the borrow checker, e.g.

```
    bb4: {
        _2 = const State::B;
        goto -> bb1;
    }
```

However, it might be adventageous to desugar by "inlining" the `match` in certain cases, i.e.

```
    bb4: {
        _2 = const State::B;
        switchInt(move _2) -> [0: bb4, 1: bb3, otherwise: bb2];
    }
```

The idea is this transformation could get LLVM to generate better code (e.g. computed GOTO). The best desugaring will differ from case to case though, so will have to be determined experimentally.

### Lowering Details: `const continue value`

When encountering a `const continue value`, rather than the standard desugaring that jumps back to the top of the loop

```
    bb4: {
        _2 = const State::B;
        goto -> bb1;
    }
```

we instead desugar by "inlining" the original match

```
    bb4: {
        _2 = const State::B;
        switchInt(move _2) -> [0: bb4, 1: bb3, otherwise: bb2];
    }
```

And then perform constant propagation into the `switchInt`, so that we get

```
    bb4: {
        _2 = const State::B;
        goto -> bb3;
    }
```

The restrictions on `const continue` mean that we can always desugar to a `goto`.

### Lowering Details: `loop const match`

A `loop const match <operand>` jumps directly to the branch that matches `<operand>`. E.g.

```rust
loop match State::A {
    State::A => {
        // perform work
        const continue State::B;
    }
    State::B => {
        break 42
    }
};
```

will desugar into (roughly)


```
    bb1: {
        goto -> bb4;
    }

    bb2: {
        unreachable;
    }

    bb3: {
        StorageDead(_2);
        return;
    }

    bb4: {
        _2 = const State::B;
        goto -> bb3;
    }
```

Because the control flow is known statically, the match disappears in this case.

## Implications for borrow checking

Because `const continue` is desugared to a `goto` when HIR is lowered to MIR, the more precise control flow information is available to the borrow checker. Hence more programs are accepted that would otherwise be rejected with an error like:

```
error[E0382]: use of moved value: `owned`
  --> src/main.rs:11:18
   |
4  | let owned = Box::new(1);
   |     ----- move occurs because `owned` has type `Box<i32>`, which does not implement the `Copy` trait
5  | let mut state = State::Foo;
6  | loop {
   | ---- inside of this loop
...
11 |             drop(owned); // ERROR use of moved value: `owned`
   |                  ^^^^^ value moved here, in previous iteration of loop
```

The borrow checker already operates on basic blocks, and [can handle irreducible control flow](https://rust-lang.zulipchat.com/#narrow/channel/186049-t-types.2Fpolonius/topic/Borrow-checking.20irreducible.20control-flow.3F), so no specific changes are needed.

# Drawbacks
[drawbacks]: #drawbacks

## irreducible control flow

The `const continue` construct introduces a way of expressing [irreducible control flow](https://en.wikipedia.org/wiki/Control-flow_graph#Reducibility) in the rust surface language. As far as we know, there are no blockers (e.g. [borrow checking should be able to handle it](https://rust-lang.zulipchat.com/#narrow/channel/186049-t-types.2Fpolonius/topic/Borrow-checking.20irreducible.20control-flow.3F), but currently it is not specified that HIR to MIR desugaring can introduce irreducible control flow (this has been discussed in [#114047](https://github.com/rust-lang/rust/issues/114047)).

So while there are no blockers for this particular RFC, once you have irreducible control flow in the language there is no way back.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Let's look at alternatives in turn

## switch fall-through

In C, the "feature" of `switch` blocks automatically falling through to the following branch is used to guarantee a direct jump between two states. This idea gives rise to examples like [Duff's device](https://en.wikipedia.org/wiki/Duff%27s_device):

```c
send(to, from, count)
register short *to, *from;
register count;
{
    register n = (count + 7) / 8;
    switch (count % 8) {
    case 0: do { *to = *from++;
    case 7:      *to = *from++;
    case 6:      *to = *from++;
    case 5:      *to = *from++;
    case 4:      *to = *from++;
    case 3:      *to = *from++;
    case 2:      *to = *from++;
    case 1:      *to = *from++;
            } while (--n > 0);
    }
}
```

This fall-through behavior is often considered unintuitive, to the point that in many C code bases, such fall-throughs are explicitly labeled with a comment to call attention to the fact that the fall-through is deliberate. But this feature is a part of C for a reason: the fall-through is an unconditional jump, which is often essential for good performance.

The `loop match` proposal has two major advantages over fallthrough:

- there is no need to list branches in a particular order
- more than one next state can be reached with a direct jump

It turns out that `loop match` is fairly expressive, and can in fact express Duff's device:

```rust
// originally written by Ralf Jung on zullip
// assumes count > 0
// `one()` performs a one-byte write and increments the counters
let mut n = count.div_ceil(4);
loop match count % 4 {
  0 => { one(); const continue 3 }
  3 => { one(); const continue 2 }
  2 => { one(); const continue 1 }
  1 => { one(); n -= 1; if n > 0 { const continue 0; } else { break; } }
  _ => unreachable(),
}
```

## Labeled blocks

The dedicated programmer can use labeled blocks to simulate the C fallthrough behavior.

But labeled blocks scale poorly in the number of states: each state needs its own scope, adding at least one level of indentation.
Following the control flow is tricky, especially because there are now implicit fallthroughs between states.
Compare these semantically equivalent implementations:

```rust
fn labeled_blocks() -> Option<u8> {
    'foo {
        's3 {
            's2 {
                's1 {
                    match 1u8 {
                        1 => break 's1,
                        2 => break 's2,
                        3 => break 's3,
                        _ => break 'foo None,
                    }
                }

                // s1 logic

                // fallthrough to s2
            }

            // s2 logic

            // fallthrough to s3
        }

        break 'foo Some(42)
    }
}

fn loop_match() -> Option<u8> {
    loop match 1u8 {
        1 => continue 2,
        2 => continue 3,
        3 => break Some(42),
        _ => None
    }
}
```

Nested labeled blocks do not spark joy.

Macros can be used to tame the syntactic complexity to some extent, but that just introduces custom syntax to learn for something as fundamental to a low level programming language as a state machine. Furthermore, editor experience within macros is still not as good as for first-class language constructs.

A second limitation is that only forward jumps (from an earlier to a later branch) are possible. To go back to an earlier branch, a loop and unpredictable match are still required. Thus, `loop match` wins in brevity, expressivity and code generation quality.

## guaranteed tail calls

In C and other languages, some modern interpreters make use of guaranteed tail calls to ensure that state transitions are just a single jump.

The [wasm3](https://github.com/wasm3/wasm3) webassembly interpreter is a well-known example. Their [design document](https://github.com/wasm3/wasm3/blob/main/docs/Interpreter.md#tightly-chained-operations) describes their approach and also mentions some further prior art.

This feature request has a long history in rust, because the details are hard to get right. The current proposal is [Explicit Tail Calls](https://github.com/phi-go/rfcs/blob/guaranteed-tco/text/0000-explicit-tail-calls.md).

This [zig issue](https://github.com/ziglang/zig/issues/8220) gives three good reasons for why guaranteed tail calls don't cover all cases:

- on some targets, tail calls cannot be guaranteed (or at least LLVM currently won't)
- logic must be organized into functions, this has potential performance implications, but also stylistic ones.
- debugging of logic structured with tail calls is much more difficult than code that stays within a single stack frame

Tail calls are a useful tool, and rust should have them, but there are still use cases for `loop match`.

### zlib-rs usage report

We benchmarked an implementation using tail calls versus "loop + match" and our PoC `loop match` implementation. The results are [here](https://gist.github.com/folkertdev/977183fb706b7693863bd7f358578292). We see significant (~15%) speedups of `loop match` over tail calls in some benchmarks.

```
Benchmark 3 (80 runs): /tmp/labeled-match-len rs-chunked 4 silesia-small.tar.gz
  measurement          mean ± σ            min … max           outliers         delta
  wall_time          62.6ms ±  555us    61.7ms … 66.1ms          2 ( 3%)        ⚡- 14.0% ±  1.4%
  peak_rss           24.1MB ± 77.9KB    23.9MB … 24.1MB          0 ( 0%)          -  0.1% ±  0.1%
  cpu_cycles          249M  ± 1.87M      248M  …  263M           5 ( 6%)        ⚡- 15.4% ±  1.3%
  instructions        686M  ±  267       686M  …  686M           0 ( 0%)        ⚡- 24.9% ±  0.0%
```

In the `loop match` version we load many values to the stack explicitly, and keep them there for the full duration of the function. The tail call approach instead needs to load values from the state repeatedly. In theory LLVM might be able to remove these redundant loads, but it looks like it can't today. A `loop match` is easier to optimize by both the programmer and the compiler in this case.

## Join points

In functional languages, where closures are typically heap-allocated, non-toplevel functions can be promoted to join points. Join points are never heap-allocated (hence are cheaper to create and do not need to be garbage collected), and are able to express iteration without growing the stack. Join points were introduced in [compiling without continuations](https://pauldownen.com/publications/pldi17.pdf) to solve the performance problem of heap-allocated closures without compromising on the algebraic properties of functional languages.

Join points are implemented in at least Haskell, Lean, Koka and Roc. None of these languages have explicit syntax for a user to write a join point: programmers know the rules the compiler uses to promote a binding to a join point, and write their code so that the optimization kicks in. This is similar to how these and other languages guarantee tail-call elimination if the code is structured a certain way.

But, rust does not have the problem (heap-allocated closure) or the constraint (nice algebraic rewriting properties) of the languages where this construct is used. Closures in rust are already cheap to create and stored on the stack. Mutation and constructs like loops with breaks make applying rewrite rules of the style used in functional compilers virtually impossible already.

## Safe GOTO

The feature proposed in https://internals.rust-lang.org/t/pre-rfc-safe-goto-with-value/14470/51 touches on a lot of the same problems as this RFC.

The advantage of `loop match` is that it makes a connection between control flow and data flow. In particular with label-based proposals, it is laborious to store a state and later resume in that state.

## Computed goto

A feature of some C compilers where syntax is provided for creation of jump tables. E.g.

```c
int interp_cgoto(unsigned char* code, int initval) {
    /* The indices of labels in the dispatch_table are the relevant opcodes
    */
    static void* dispatch_table[] = {
        &&do_halt, &&do_inc, &&do_dec, &&do_mul2,
        &&do_div2, &&do_add7, &&do_neg};
    #define DISPATCH() goto *dispatch_table[code[pc++]]

    int pc = 0;
    int val = initval;

    DISPATCH();
    while (1) {
        do_halt:
            return val;
        do_inc:
            val++;
            DISPATCH();
        do_dec:
            val--;
            DISPATCH();
        do_mul2:
            val *= 2;
            DISPATCH();
        do_div2:
            val /= 2;
            DISPATCH();
        do_add7:
            val += 7;
            DISPATCH();
        do_neg:
            val = -val;
            DISPATCH();
    }
}
```

[source](https://eli.thegreenplace.net/2012/07/12/computed-goto-for-efficient-dispatch-tables)

There are two reasons one might use a computed goto

- get better code generation than the standard "loop + match"
- indexing into an array of future states is more natural than a match

However, `loop match` promises even better code generation than the jump table that computed goto produces in cases where targets are compile-time known, and has roughly similar ergonomics, e.g.

```rust
macro_rules! dispatch() {
    () => {
        let temp = code[pc]; // or .get_unchecked
        pc += 1;
        temp
    }
}

loop match dispatch!() {
    DO_HALT => break 'top val,
    DO_INC => {
        val += 1;
        continue dispatch!();
    DO_DEC => {
        val -= 1;
        continue dispatch!();
    }
    DO_MUL2 => {
        val *= 2;
        continue dispatch!();
    }
    DO_DIV2 => {
        val /= 2;
        continue dispatch!();
    }
    DO_ADD7 => {
        val += 7;
        continue dispatch!();
    }
    DO_NEG => {
        val = -val;
        continue dispatch!();
    }
    _ => unreachable!(), // or unreachable_unchecked()
}
```

In the current PoC implementation each `continue` will duplicate the match, leading to the branch prediction behavior that makes computed goto attractive. However, it is not currently clear that this desugaring will be kept for non-const `continue <operand>`.

## improve MIR optimizations

In theory, more sophisticated analysis of the MIR should be able to optimize the "loop + match" pattern into a collection of unconditional jumps. We've seen that it's not capable of performing this optimization today, but if it could, then from a performance perspective maybe `loop match` would not be needed.

While improvements to rust's MIR passes (or even a whole new IR that is better suited to optimization) are certainly possible, limitations are:

- the implementation complexity
- the compile time cost
- analysis is fragile
- this transform may not be adventageous in general

In contrast

- loop match has a straightforward desugaring
- the transformation is syntax-driven, and therefore nicely bounded
- programmers can write their code in such a way that they can be confident the desugaring to a `goto` kicks in
- the (expert) programmer definitely wants the desugaring into a `goto`

So, `loop match` is a solid way to make progress on better codegen. Improved optimizations on MIR are also very welcome, but never entirely remove the need for `loop match` from a programmer's perspective.

## recognize "loop + match" and optimize

In theory it is possible to internally recognize and rewrite a "loop + match" expression into a `loop match`. With this approach, no changes to language syntax are needed.

A fundamental problem with this approach is a change in drop order:

```rust
let mut state = 0;
'label: loop {
    match state {
        0 => {
            let x = vec![1,2,3];
            state = 1;
            // drop of `x` gets inserted between state update and jump
            continue 'label;
        }
        _ => ...
    }
}

// versus if you rewrite to `loop match`

loop match 0 {
    0 => {
        let x = vec![1,2,3];
        // drop of `x` happens before the state update
        continue 1;
    }
    _ => ...
}
```

Beyond that, the analysis for recognizing "loop + match" will likely be complex and fragile. Part of the appeal of `loop match` is that the desugaring rules are simple and deterministic. Using a `loop match` is intentional, and signals that something subtle is going on: for readers and future reviewers it is clear that the `loop match` desugaring is desired and potentially crucial for the code to perform well.

## introduce just `continue <operand>`

Do we really need `loop match`? We could instead allow `continue <operand>` inside of `loop { match _ { ... } }`.

An advantage of this approach is that one can define macros that have the loop's label in scope:

```rust
let mut state = 0;

'label: loop {
    macro_rules! foo {
        () => {
            continue 'label (state + 1)
        }
    }

    match state {
        1 => foo!(),
        2 => foo!(),
        _ => todo!(),
    }
}
```

The main downside here is that it is really subtle that the `continue <operand>` expression updates the match scrutinee in the next iteration.

## don't introduce irreducible control flow

It seems possible to delay the desugaring of a `const`-annotated state transition to a goto until after borrow checking. In that case we'd annotate the relevant `goto`s to the top of the loop with a "shortcut", the location where they will actually jump to.

```
    bb1: {
        _3 = discriminant(_2);
        switchInt(move _3) -> [0: bb4, 1: bb3, otherwise: bb2];
    }

    bb4: {
        _2 = const State::B;
        goto -> bb1; // <- would be annotated with "please make this a `goto -> bb3` after borrow checking"
    }
```

The advantages of having more accurate borrow checking, and accepting more valid programs, are compelling to me, but this more limited solution here could absolutely work from just a performance perspective.

## Syntax Squables

A previous version of this RFC proposed labeled match:

```rust
fn labeled_match() -> Option<u8> {
    'label: match 1u8 {
        1 => continue 'label 2,
        2 => continue 'label 3,
        3 => break 'label Some(42),
        _ => None
    }
}
```

Two problems were identified with this syntax

- it is unintuitive that a `match` can loop
- labels are not first-class; we'd rather not use them in more places

The `loop match` syntax is easily searchable, and gives a good intuition for what the construct does (it loops and matches). A downside of this syntax is that it is both a loop and a match in one, so error messages have to be revised to be accurate (e.g. how to report that a `loop match` is non-exhaustive).

The `loop const match` and `const continue` variants were introduced later, because:

- they demand that the jump target is known at compile time
- they guarantee the desugaring to a MIR `goto`: this has borrow checker implications

The usage of `const` provides a specific place to document the conditions and behavior, and makes it possible to use `loop match` for ergonomics reasons even if the next state is not statically known.

The original proposal was `static continue`, which is fine but `const continue` gives a better intuition for what the operand should be.

### Using a macro to defer syntax choices

Another option is to defer the question on what the right syntax is by using macros. A similar approach has been taken with `addr_of!(x)` → `&raw x` and `matches!(x, p)` → `x is p`. For instance:

```rust
finite_state_machine! {
    goto dyn count % 4;
    0 => {
        one();
        goto 3;
    }
    3 => {
        one();
        goto 2;
    }
    2 => {
        one();
        goto 1;
    }
    1 => {
        one();
        n -= 1;
        match n {
            0 => break,
            _ => goto 0,
        }
    }
    _ => unreachable!(),
};
```

Such a macro needs serious compiler support: it needs all the backend features (lowering to a MIR `goto`, validating that the value of a non-dyn `goto` is static promotable), and likely also some custom error messages around the `goto` "keyword", or just the syntax of this macro in general.

## Why `loop match` is the best solution

In summary, `loop match`:

- is a straightforward combination of `loop` and `match`. In its basic form it is just syntax sugar, and should not present issues for beginners. The more advanced `const continue` is a fairly specific tool, that beginners are unlikely to encounter, and is straightforward to look up.
- does not introduce arbitrary control flow (like general `goto`) or surprising implicit control flow (like `switch` fallthrough in C and descendants). The mechanism based on pattern matching fits nicely into how rust works today.
- is not blocked on LLVM, and can be implemented entirely in rustc, providing benefits to all code generation backends. The implementation and maintenance effort is small, because infrastructure that is already in place for labeled loops and blocks is reused.
- accepts more valid programs, by providing more accurate CFG information to the backend.

The codegen characteristics provided by `const continue` are essential in real-world programs, like [`zlib-rs`](https://github.com/memorysafety/zlib-rs). Improvements to MIR optimizations are welcome, but unlikely to reliably give the desired codegen. The inability to generate efficient code actively limits the adoption of rust in domains where performance is key. Without a feature like this, it is effectively impossible to beat C in certain important cases.

# Prior art
[prior-art]: #prior-art

This idea is taken fairly directly from zig.

The idea was first introduced in [this issue](https://github.com/ziglang/zig/issues/8220) which has a fair amount of background on how LLVM is not able to optimize certain cases, reasoning about not having a general `goto` in zig, and why tail calls do not cover all cases.

[This PR](https://github.com/ziglang/zig/pull/21257) implements the feature, and provides a nice summary of the feature and what guarantees zig makes about code generation.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

**What parts of the design do you expect to resolve through the RFC process before this gets merged?**

- introduction of `loop const? match <scrutinee> { ... }`, and `const? continue <operand>` syntax
- when a `const continue` operand is accepted (i.e. under what conditions we can/want to guarantee a MIR `goto` will be produced)
- the semantics of `loop const match` and `const continue`: these constructs have borrow checker implications and introduce irreducible control flow in the surface language

The RFC text provides background on why this feature is needed for improved code generation, but from the language perspective, only the above three elements are required.

**What parts of the design do you expect to resolve through the implementation of this feature before stabilization?**

The happy path of HIR to MIR specialization is clear, but there are some questions around what to do when plain `continue value` does not obviously match a branch. Detailed benchmarking, and investigating the interaction with other MIR optimizations will be required to figure out what the best approach is in all cases.

We may also want to desugar differently based on the optimization level (in particular when optimizing for binary size). Again this will require experimentation.

**What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?**

None so far

# Future possibilities
[future-possibilities]: #future-possibilities

## Relax the constraints on the `loop const match` and `const continue` operands

We currently don't know how to do this exactly, but it seems feasible to accept (some) values where only part of the pattern is known, e.g.

```rust
loop match None {
    None => {
        println!("None");
        const continue Some(unsafe { not_comptime_known() });
    }
    Some(false) => {
        println!("Some(false)");
        const continue Some(false);
    }
    Some(true) => {
        println!("Some(true)");
        break;
    }
}
```

Some extensions are just hard to specify with the vocabulary we currently have (these partially-known values are not const, or static promotable), other cases would expose the order in which patterns are evaluated, and so this order would have to be stabilized in order to support them for the const variants of `loop match`.

## Computed GOTO

Depending on how the experiments around the exact desugaring strategy work out, we might be able to lower a `continue value` on an unknown value into a jump table. The current PoC has this behavior, but further experimentation is needed to establish if the codegen is actually good, and how the downsides (e.g. larger binary size) can be managed.

This [recent thread](https://internals.rust-lang.org/t/idea-for-safe-computed-goto-using-enums/21787)  has some further ideas.

# Thanks

- @bjorn3 for writing the PoC implementation
- @joshtriplett, @jackh726, folks at GOSIM 2024, and others for providing feedback
