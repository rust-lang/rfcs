- Feature Name: rust_has_provenance
- Start Date: 2023-11-22
- RFC PR: [rust-lang/rfcs#3559](https://github.com/rust-lang/rfcs/pull/3559)
- Rust Issue: [rust-lang/rust#121243](https://github.com/rust-lang/rust/issues/121243)

# Summary
[summary]: #summary

Pointers (this includes values of reference type) in Rust have **two** components.
* The pointer's "address" says where in memory the pointer is currently pointing.
* The pointer's "provenance" says where and when the pointer is allowed to access memory.

(This is disregarding any "metadata" that may come with wide pointers, it only talks about thin pointers / the data part of a wide pointer.)

Whether a memory access with a given pointer causes undefined behavior (UB) depends on both the address and the provenance:
the same address may be fine to access with one provenance, and UB to access with another provenance.

In contrast, integers do **not** have a provenance component.

Most of the rest of the details, such as a specific provenance model, are intentionally left unspecified.

This RFC very deliberately aims to be as **minimal** as possible, to just get the entire Rust Project on the "same page" about the long-term future development of the language.

# Motivation
[motivation]: #motivation

"Shared references (and pointers derived from them) are read-only" is a well-established principle in Rust.
The presence of provenance follows directly from that principle, as can be seen by the following example:

```rust
fn main() { unsafe {
    let mut x = 5;
    // Setup a mutable raw pointer and a shared reference to `x`,
    // and derive a raw pointer from that shared reference.
    let ptr = &mut x as *mut i32;
    let shrref = &*ptr;
    let shrptr = shrref as *const i32 as *mut i32;
    // `ptr` and `shrptr` point to the same address.
    assert_eq!(ptr, shrptr);
    // And yet, while writing to `ptr` here is perfectly fine,
    // the next line is UB!
    shrptr.write(0); // alternative: `ptr.write(0);`
} }
```

If you agree that this program has UB while the indicated alternative is permitted, then as a logical consequence you must agree that Rust pointers have provenance.
After all, `ptr` and `shrptr` are identical in terms of their representation in the compiled program.
The only way for there to be a difference between them is for pointers to carry "something extra", beyond the address, that indicates how they may or may not be used.
This "something extra" is what we call *provenance*.

## Optimizations

Provenance is useful because it allows powerful optimizations.

Many (most?) optimizations done by compilers require some form of *alias analysis*. This is an analysis that reports when two memory operations might alias each other. Alias analysis benefits greatly from notions of provenance since this generally means there is more UB and more information with which to justify optimizations. For example, consider the following program:

```rust
fn main() { unsafe {
    use core::ptr::{self, addr_of_mut};

    let mut p1 = 42u8;
    let mut p2 = 42u8;
    let p1_ptr = addr_of_mut!(p1).wrapping_add(1);
    let p2_ptr = addr_of_mut!(p2);
    if ptr::eq(p1_ptr, p2_ptr) {
        *p1_ptr = 10; // <-- assignment 1
        //*p2_ptr = 10; // <-- (alternative) assignment 2
        // This can be optimized only with provenance:
        println!("{}", p2);
    }
}}
```

The indicated [alternative](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=a4dcbe22b8fe94d113ff6a27c3d43fc0), where the second assignment is enabled and the first assignment is disabled, "obviously" is well-defined since it just creates a pointer to `p2` and then writes to it.
That program will hence either print nothing or print 10, but never have UB.
Since `p1_ptr` and `p2_ptr` are equal, assuming "pointers are just integers" (i.e., assuming that there is *no* pointer provenance, or at least it is not relevant for program behavior), we can replace one by the other, and therefore the given program must also be allowed and have the same behavior: print nothing or print 10, but never have UB.

However, from the perspective of alias analysis, we want this program to have UB: looking at `p2` and all pointers to it (which is only `p2_ptr`), we can see that none of them are ever written to, so `p2` will always contain its initial value 42.
Therefore, alias analysis would like to conclude that if this program prints anything, it must print 42, and replace `println!("{}", p2)` by `println!("{}", 42)`.
After this transformation, the program might now print nothing or print 42, even though the original program would never print 42.
Changing program behavior in this way is a violation of the "as-if" rule that governs what the compiler may do.
The only way to make that transformation legal is to say that the given program has UB.
The only way to make the given program have UB, while keeping the alternative program (that writes to `p2_ptr`) allowed, is to say that `p1_ptr` and `p2_ptr` are somehow different, and writing through one of these pointers is *not* like writing through the other.
Given that the address the pointers point to is identical, this means there must be "something extra" beyond the address that is different between them: `p1_ptr` has to remember that it "belongs to" `p1`, not `p2`, and therefore using it to write to `p2` is UB.
In other words, the pointer carry along their provenance, and pointer provenance matters for whether programs have UB or not.
The given program has UB, but the [alternative program](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=a4dcbe22b8fe94d113ff6a27c3d43fc0) does not.[^miri]

[^miri]: If you try running the given program in Miri, you might be surprised to see that Miri does not report UB. This is because the UB can only be detected when `ptr::eq(p1_ptr, p2_ptr)` is true, and with Miri's randomized allocator, that is unlikely. [Here is another version](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=88aaf7200e962050c140709fac24042c) that tries multiple possible offsets between `p1` and `p2`, and reliably triggers UB under current versions of Miri.

This optimization is performed by both [GCC](https://godbolt.org/z/G3jYEnWx6), [clang](https://godbolt.org/z/cr7h6hhqf), and [ICC](https://godbolt.org/z/14b1d16Gc):
in all cases, the program prints `42`, showing that the initial value of `p2` is printed, not the value that was written just above the `print` call -- despite the fact that that write definitely stores to the same address that `print` is printing from.
I wasn't able to find a website that can build and run code with MSVC, but [this assembly](https://godbolt.org/z/dzPz8WM7Y) seems to indicate that it, too, would call `print` with an argument of `42`, and thus is using provenance.
This is not a new phenomenon either; it goes back at least to [GCC 4.6.4](https://godbolt.org/z/Yx6f389Gf) (released in 2013) and [clang 3.4.1](https://godbolt.org/z/nnhn6fdnj) (released in 2014).
This demonstrates that both of them implement a language that has pointer provenance.[^cstandard]

[^cstandard]: What the compilers do is justified by the C standard: `p1_ptr` is a "one past the end" pointer, and those may not be written to. However, this just demonstrates that the C standard has a notion of provenance built-in without acknowledging that fact; without provenance, there would be no way for the standard to distinguish the "one past the end" pointer `p1_ptr` from the completely valid `p2_ptr`. After all, the alternative program where the assignment writes to `p2_ptr` instead of `p1_ptr` is unambiguously well-defined -- and both pointers point to the same address.

[This blog post](https://www.ralfj.de/blog/2020/12/14/provenance.html) uses a variant of the above example to show what can go wrong when the interactions of provenance and compiler optimizations are being ignored.

#### Optimizations for reference types

Similarly, it has long been desirable for it to be sound to optimize code like this:

```rust
fn foo(x: &mut i32) -> i32 {
    *x = 10;
    bar();
    *x
}
```

It's very difficult to see how to make this optimization sound without provenance. Ralf J. has [attempted](https://www.ralfj.de/blog/2017/07/17/types-as-contracts.html) such a model in the past, but it was unsuccessful in a number of ways: the optimizations it allows are fairly weak (replacing `bar` by an unknown block of code within the same function would already inhibit the optimizations), while at the same time the model was incompatible with common unsafe code patterns (to the extent that even the standard library needed a long allowlist to make the Miri test suite pass).

In contrast, Ralf's successor model [Stacked Borrows](https://github.com/rust-lang/unsafe-code-guidelines/blob/a4a6e5f28b6542da759db247db7db8b34d5f0ead/wip/stacked-borrows.md) and the more recent [Tree Borrows](https://perso.crans.org/vanille/treebor/) do enable powerful optimizations for references while being compatible with the majority of existing unsafe code.
Both of these models heavily rely on provenance.

## LLVM

LLVM IR (despite its lack of a clear spec for provenance) recognizes a notion of allocation-level provenance. This is apparent in two ways:
- `getelementptr` (without `inbounds`) produces a pointer that is still "tied to" its original allocation. Even if its address is now inbounds of another allocation, it would be UB to access any but the original allocation via this pointer. This can only be explained by saying that the pointer "remembers" the allocation it belongs to in a way that is independent of its actual address -- a classic example of provenance.
- The specification for `noalias` explicitly talks about "pointers derived from another pointer". It doesn't specify how "derived from" is defined, but the most plausible explanation is via some form of provenance that "remembers" which `noalias` pointer a pointer is derived from.

Compiling Rust to LLVM IR if Rust does not recognize provenance is likely to be impossible. We'd probably have to insert a `black_box` after every allocation and every memory access, and it's not clear that that is enough. As far as I know there is no option to turn this off, and the assumptions are sufficiently widespread that it is unlikely that we could convince upstream to add one.

## Integers do not have provenance

While pointers have provenance for the reasons stated above, integers do not.
This means that values of integer type are fully determined by the bits one can observe during execution of a compiled program.[^determined]
(This is in contrast to other types where seeing the bits is insufficient to reconstruct the abstract value, since one cannot deduce if a byte is initialized or which provenance a pointer carries.)
This is crucial to obtain all the usual arithmetic operations on integers: integers with provenance have difficulty supporting transformations such as `x * 0 --> 0` (which forgets the fact that the final value used to syntactically depend on `x`), and they are fundamentally incapable of doing optimizations like the following:
```rust
if x == y {
  // in this block, replace `x` by `y` or vice versa
}
```

[^determined]: Beyond the contents of this RFC, this assumes that integers cannot be uninitialized, which current codegen relies on in the form of `noundef` attributes.

However, as a low-level systems language, Rust still needs some way to store and copy "memory with arbitrary content", including pointers that can have provenance.
Popular belief says that an array of `u8` is suited for this purpose, but that is not true, because of provenance as stated above.
In fact, "arbitrary content" may be "uninitialized memory", and `u8` must be initialized, so this is already not true even when disregarding provenance.
However, `MaybeUninit<u8>` *is* suited for this purpose.
It already must be able to store and copy uninitialized memory; there is no downside to also letting it store and copy pointers with provenance.

## Descriptive vs prescriptive provenance

Note that "provenance" is a somewhat unfortunate term.
Specifically, there are two completely distinct forms of provenance, which we might call "prescriptive" and "descriptive".

"Descriptive" provenance is purely a means of doing program analysis.
For instance, consider the following code snippet:
```rust
let x = if b { y/2 } else { z+42 };
```
Program analysis might want to track which variables can influence the value of `x`, and this is often called "provenance".
In our example, `x` would have provenance of `{y, z}`, indicating that those are the two variables that can affect the value of `x`.
However, this kind of provenance is purely *descriptive*, it just states facts about program executions.
This is just a way of talking about data dependencies (and possibly control dependencies).
The language standard would never even mention this form of provenance; a compiler would justify the correctness of its provenance analysis by relating them to the semantics specified in the standard.
It is safe to "forget" descriptive provenance during analysis (and it doesn't exist outside analysis to begin with); that just means the compiler cannot do provenance-based optimizations on the affected values.
Programmers do not have to think about descriptive provenance ever when judging the correctness of their code.
Descriptive provenance can never make a program UB!

This is in strong contrast to *prescriptive* provenance, the kind of provenance that this RFC is about.
Prescriptive provenance is part of the language specification, and it *can* make a program UB.
This means it exists outside of program analysis, even during program execution, in the sense that it determines whether that execution has UB or not.[^exists]
When a language has requirements like "using a pointer with the wrong provenance to access some address in memory is UB", it is *not* safe to drop provenance during program execution -- provenance now becomes the permission to access some region of memory, and dropping that permission means losing the access rights![^erase]
This kind of provenance is very similar to the memory capabilities that capability machines like [CHERI](https://www.cl.cam.ac.uk/research/security/ctsrd/cheri/) are tracking in their wide pointers.
However, prescriptive pointer provenance does not have to have any real hardware counterpart; similar to the distinction of [initialized and uninitialized memory](https://www.ralfj.de/blog/2019/07/14/uninit.html), it can also exist as a "purely abstract" part of the abstract machine -- very relevant for program correctness and compiler optimizations, but not observable in the compiled programs.
Sanitizers and undefined behavior detectors like [Miri](https://github.com/rust-lang/miri/) make that abstract state concrete to be able to detect the UB governed by the rules of the abstract machine.

[^exists]: This is the same sense in which the distinction between initialized and uninitialized memory "exists" during program execution, even though it cannot be observed on most hardware.
[^erase]: It is of course still possible to erase provenance during compilation, *if* the target that we are compiling to does not actually do the access checks that the abstract machine does. What is not safe is having a language operation that strips provenance, and inserting that in arbitrary places in the program.

The point of this RFC is that Rust has *prescriptive* provenance.
The author is not aware of cases of descriptive provenance that actually use the term "provenance"; usually people simply talk about data/control dependencies.
So while the term "provenance" might initially raise wrong expectations, there's also no pressing need to pick a different term.
Ultimately, wrong expectations will ensue with pretty much any name, since few people actually expect anything like prescriptive provenance to exist.
(This includes the author of this RFC, who was firmly anti-prescriptive-provenance around 2017, but has since come to the conclusion that there's no credible alternative.)

*Historical note:* The author assumes that provenance in C was originally intended to be purely descriptive.
However, the moment compilers started doing optimizations that exploit undefined behavior depending on the provenance of a pointer, provenance of de-facto-C became prescriptive.
A lot of the confusion around provenance arises from the fact that many people still think it is purely descriptive.
They will hence accept both "we do provenance-based alias analysis" and "pointers are just integers" as true statements, not realizing that these statements are contradicting each other.
The standard has not (yet) been updated to clarify this, but in 2022 the committee has accepted a Technical Specification that does explicitly state that C has prescriptive provenance.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This isn't as big a deal as it might seem, since provenance is not an issue that ever needs to be considered within safe Rust code.

Should this RFC be accepted, the plan is to stabilize some form of [strict provenance APIs](https://doc.rust-lang.org/nightly/std/ptr/index.html#strict-provenance).
That will allow unsafe code authors to deal with provenance in a very explicit way.

The existing "escape hatch" of using pointer-to-integer and integer-to-pointer casts will still be supported.
However, it is currently unclear how to specify these operations in a way that both satisfies the requirements imposed by their intended use and permits the desired optimizations of unrelated program constructs.
This RFC and strict provenance do not change anything about the status of integer-to-pointer casts: both before and after this RFC, these casts lack a proper specification.
The benefit of strict provenance is that it enables *some* code (such as pointer bit packing) to be written with clearly specified, well-understood operations, without relying on integer-to-pointer casts.

The other big change that unsafe code has to be aware of follows from the fact that integers do *not* have provenance.
This means that a pointer, in general, carries more information than can be captured by an integer type.
For instance, transmuting a raw pointer to an array of `u8`, and then transmuting it back, does *not* restore the original pointer!
(This RFC does not specify what exactly that roundtrip does. Unsafe code authors should conservatively assume that it is UB.)
Code that wants to store data of arbitrary type needs to use an array of `MaybeUninit<u8>` instead.
The `MaybeUninit<u8>` type is guaranteed to preserve provenance (and (un)initialization state) of all its representation bytes.
(And `u8` is not a special case here, this works for all integer types and more generally for all types without padding bytes. It [gets tricky](https://github.com/rust-lang/rust/issues/99604) for `MaybeUninit<T>` when `T` itself has padding bytes.)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Within the Rust Reference, the section on "Pointer types" is extended to say that pointers have provenance, i.e., two pointers can be "different" (in terms of the program semantics) even if they point to the same address.
The same goes for other types that can carry pointer values: references and function pointers.

On the "Behavior considered undefined" page, the definition of "Dangling pointers" is adjusted to say:

> A reference/pointer is "dangling" if it is null or if not all of the bytes it points to may be accessed with its provenance. In particular, all the bytes it points to must be part of the same live allocation.

The strict provenance API will be stabilized to provide unsafe code with the ability to maintain pointer provenance more explicitly; the details of that API will be determined by T-libs-api in collaboration with T-opsem.

Furthermore, the section on "Integer types" is extended to say that integers do *not* have provenance, and therefore transmuting (via `transmute` or type punning) from a pointer to an integer is a *lossy* operation and might even be UB.
(The exact semantics of that operation involve some subtle trade-offs and are not decided by this RFC.)

Finally, `MaybeUninit<T>` is documented to preserve provenance ([at least in non-padding bytes of `T`](https://github.com/rust-lang/rust/issues/99604)).
(Eventually we might want to guarantee this for all `union`, but for now just guaranteeing it for `MaybeUninit` seems sufficient.)

# Drawbacks
[drawbacks]: #drawbacks

The biggest downside of provenance is complexity. The existence of provenance means that authors of unsafe code must always not only be concerned with whether the pointer they have points to the right place, but also whether it has the right provenance (in practice, this means "was obtained in the right way"). Not having provenance ensures that this is never a problem -- all pointers that point to the right address are equally valid to use.

The other main drawback is the lack of proper treatment of provenance in LLVM, our primary codegen backend.
LLVM suffers from various long-standing provenance-related bugs ([[1]](https://github.com/llvm/llvm-project/issues/34577), [[2]](https://github.com/llvm/llvm-project/issues/33896)), and there is currently no concrete plan for how to resolve them.
The opinion of the RFC author is that LLVM needs to stop using pointer comparisons in GVN, and it needs to stop folding ptr2int2ptr cast roundtrips.
Those optimization cannot be justified with any form of provenance, and LLVM's alias analysis cannot be justified without some form of provenance.
Furthermore, LLVM needs to decide whether the `iN` type carries provenance or not.
[This proposal](https://discourse.llvm.org/t/a-memory-model-for-llvm-ir-supporting-limited-type-punning/61948) describes how an `iN` type with provenance could work.
If `iN` does not carry provenance, then a ["byte" type](https://lists.llvm.org/pipermail/llvm-dev/2021-June/151521.html) that *does* carry provenance is required, as without such a type it would be impossible to load and store individual bytes (or in general, anything but a ptr-sized chunk of memory) in a provenance-preserving manner.
LLVM has been stuck in this limbo (various proposals but no consensus on how to proceed) for a while, without visible recent progress.
If LLVM ends up accepting either of these proposals, it will be entirely compatible with this RFC.
If LLVM makes some different choice, that might be incompatible with Rust's choices.
However, it's not possible to specify Rust in a way that is compatible with "whatever LLVM will do".
There has been no progress on these questions on the side of the LLVM project for many years (as far as the author is aware), and no concrete proposals aside from the ones sketched above, so there are only two options: (a) wait until LLVM does something, and then do something compatible in Rust, or (b) do something that makes sense for Rust, and if eventually there is movement on the LLVM side, work with them to ensure Rust's needs are covered.
(a) means indefinitely blocking progress on pressing questions in the Rust semantics, so this RFC takes the position that we should do (b).
(To the author's knowledge, GCC is not in a better position, and it suffers from [similar bugs](https://gcc.gnu.org/bugzilla/show_bug.cgi?id=82282), so we can't use their semantics for guidance either.)

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Almost all reasonably usable compiler backends use *some form* of provenance logic when optimizing code.
(The one exception we are aware of is cranelift, but that is not currently suited as a backend for release builds -- and it is unlikely to ever be suited for release builds unless it starts making use of provenance.)
There essentially is no known alternative to having provenance in some form.

One often-suggested alternative is to rely on allocator non-determinism:
unrelated code cannot "guess" the address of a memory allocation that was not "exposed", and therefore we can still optimize accesses to this allocation.
This actually works for some cases, and can even be made to work [in combination with a finite address space](https://research.ralfj.de/twinsem/twinsem.pdf), albeit the semantics already start looking rather unusual at that point.
However, all of the examples in the "motivation" section were chosen to *not* be resolved by allocator non-determinism.
If we want to do these optimizations (and we are already doing some of them today), we need provenance.

There is some possibility for alternative designs around what happens on pointer-to-integer transmutation: (1) they could act like pointer-to-integer casts, or (2) they could be outright UB, or (3) they could strip the provenance from the pointer to yield a valid integer, but the provenance has been irreversably lost.
For (1), making it work like a pointer-to-integer cast is problematic since pointer-to-integer casts [are side-effecting operations when considering provenance](https://www.ralfj.de/blog/2022/04/11/provenance-exposed.html), and as such cannot be removed even if their result is unused.
Making all transmutation sites (which includes every load from memory) possibly side-effecting that way would be a disaster for optimizations (it would prohibit elimination of dead loads), so option (1) seems infeasible.
However, from an unsafe code correctness perspective, the RFC is forward-compatible with eventually choosing option (1), should it turn out that it is feasible after all.
For (2), the benefit of that option is that it allows less code and thus reduces the risk of Rust semantics being incompatible with whatever semantics LLVM ends up using.
However, making the cast UB in MIR semantics is actually bad from an optimization perspective: it would imply that *adding* provenance to a byte can introduce UB, which causes problems for some optimizations that transform the program in a way that a pointer in the final program has "more provenance" than in the original program.
To avoid these problems, an optimizing IR should declare pointer-to-integer transmutation to be UB-free, as in option (3).
That said, (2) would still be a valid option for surface Rust, so this RFC deliberately leaves that question undecided.

# Prior art
[prior-art]: #prior-art

* "[N3057: A Provenance-aware Memory Object Model for C](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3057.pdf)"
  describes how the C standard is attempting to fit provenance concepts into C.
  This [technical specification](https://www.iso.org/standard/81899.html) has been accepted unanimously by the C standards committee, but is not (yet) part of the official ISO standard.

<details><summary>C committee minutes</summary>

[2022-01-31 Final Meeting Minutes](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n2991.pdf):

Straw poll: Does WG14 wish to see TS6010 working draft (N2676 or something similar) in some future version of the standard?<br>
21/0/1. Clear consensus.<br>
Straw poll: Does WG14 wish to see TS6010 working draft (N2676 or something similar) in C23?<br>
10/8/5. Clear indication people think this is important.<br>
Straw poll: (Opinion) Is WG 14 willing to move TS 6010 to DTS ballot as it stands now?<br>
19/1/3. The committee is OK to move forward.<br>
(the numbers are yes/no/abstain)

[2022-05-16 - 2022-05-20 Final Meeting Minutes](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3036.pdf):

Straw poll: (decision) Does WG14 want to move to a DTS ballot for TS 6010?<br>
Result: 18-0-0 (consensus)<br>
Uecker: would like to mention that other languages, especially Rust, are adopting this, so now is a useful time to progress.<br>

[2023-01-23 - 2023-01-28 Final Meeting Minutes](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3153.pdf):

Keaton: any objections to a CD ballot for TS 6010?<br>
(none, unanimous consent)<br>
DECISION: Gustedt’s document will go to SC22 and start the two-month ballot process this week.<br>
One month available if needed for ballot resolution.<br>
Sewell: can we start with ISO working in parallel?<br>
Keaton: yes, ISO has volunteered to start its review early.<br>
ACTION: Keaton to submit TS 6010 to ISO early.<br>
ACTION: Gustedt to make up an N-document for TS 6010.<br>

(That document became [N3057](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3057.pdf).)

</details>

### Prior discussion in Rust

* The question of provenance has been discussed for many years. See for instance the [provenance label in the UCG](https://github.com/rust-lang/unsafe-code-guidelines/issues?q=is%3Aissue+label%3AA-provenance), and the [strict provenance discussion](https://github.com/rust-lang/rust/issues/95228).
* There was a 2022-10-05 [lang team design meeting](https://github.com/rust-lang/lang-team/blob/c8f61dd9d933091b0487153d9db49034f8fa1002/design-meeting-minutes/2022-10-05-provenance.md) on this subject. The most relevant parts of those meeting notes were used as the starting point for this RFC.
* This RFC was discussed [on Zulip](https://rust-lang.zulipchat.com/#narrow/stream/136281-t-opsem/topic/Pre-RFC.3A.20Rust.20Has.20Provenance).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

All the particulars about the exact provenance model are largely still undetermined.
This is deliberate; the RFC discussion should not attempt to delve into those details.

The appropriate standard library API functions to let programmers correctly work with provenance (strict provenance APIs) are not yet finalized; their exact shape can be left to T-libs-api in collaboration with T-opsem.

There might be a better name than "provenance".
But (for reasons discussed [above](#descriptive-vs-prescriptive-provenance)), it's not an entirely bad term either.
Ultimately, the biggest hurdle is the concept itself, not its name.

# Future possibilities
[future-possibilities]: #future-possibilities

Future RFCs will define more specifically how provenance works in Rust.
Two concrete proposals for such provenance models are [Stacked Borrows](https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md) and the more recent [Tree Borrows](https://perso.crans.org/vanille/treebor/).
