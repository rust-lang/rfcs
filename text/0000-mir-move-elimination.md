- Feature Name: N/A
- Start Date: 2026-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary

This RFC proposes changes to Rust's operational semantics and MIR representation to enable elimination of unnecessary copies of local variables. Specifically, it makes accessing memory after a move undefined behavior, and redefines the allocation lifetime of local variables to be tied to their initialized state rather than their lexical scope. Finally, it introduces a new MIR optimization pass which exploits these guarantees to eliminate copies between locals when it is safe to do so.

## Motivation

Consider the following Rust code which constructs an outer object containing an inner object.

<details>

<summary>Rust code</summary>

[Godbolt](https://rust.godbolt.org/z/q5hcWafbK)

```rust
struct Inner {
    array: [i32; 5],
}

impl Inner {
    fn new(val: i32) -> Self {
        let mut x = Inner { array: [val; 5] };
        x.init();
        x
    }

    #[inline(never)]
    fn init(&mut self) {
        // ...
    }
}

struct Outer {
    inner: Inner,
}

impl Outer {
    fn new(val: i32) -> Self {
        let mut x = Outer {
            inner: Inner::new(val),
        };
        x.init();
        x
    }

    #[inline(never)]
    fn init(&mut self) {
        // ...
    }
}

fn main() {
    let mut foo = Outer::new(123);
    // ...
}
```

</details>


The construction of `Outer` involves several function calls which create local objects, mutate them and then return them. The MIR produced by rustc copies the 20-byte array **4 times** before the array reaches its final location as a local variable in `main`.

LLVM is able to eliminate 2 of these copies, but it fundamentally cannot do more because the LLVM IR produced by rustc forbids `main`, `Outer::init` and `Inner::init` from observing the same address.

Contrast this with the equivalent code in C++:

<details>

<summary>C++ code</summary>

[Godbolt](https://godbolt.org/z/fKxf5EhY6)

```c++
struct Inner {
    int array[5];

    Inner(int val): array{val, val, val, val, val} {
        this->init();
    }

    void init();
};

struct Outer {
    Inner inner;

    Outer(int val): inner(val) {
        this->init();
    }

    void init();
};

int main() {
    Outer foo(123);
    // ...
}
```

</details>

In C++, objects are always constructed at their final location in memory. C++ doesn't have a concept of implicit copies/moves like Rust does. Instead, all copies are explicit and involve calling a copy constructor which creates a *new* object at the destination address. This means that when the constructor for `Inner` is called, `this` already points to the local variable `foo` in `main`. As a result, the resulting assembly code contains no copies.

The inability of Rust to eliminate these copies requires awkward workarounds to avoid a performance hit or excessive stack usage (which could lead to stack overflows), usually in the form of "deferred initialization". This involves creating an object in an uninitialized state at its final location and then manually initializing it, often using unsafe code.

As an example, the `brie-tree` crate needs to use this pattern ([1](https://github.com/Amanieu/brie-tree/blob/5b0a72fcf66dc12e4754f387794afe59167bbc3b/src/lib.rs#L186-L187) [2](https://github.com/Amanieu/brie-tree/blob/5b0a72fcf66dc12e4754f387794afe59167bbc3b/src/cursor.rs#L1069-L1076)) to avoid a 15% hit in the performance of B-Tree insertion.

## Status quo

```rust
struct Foo([u8; 100]);

unsafe extern "C" {
    safe fn observe(b: *mut Foo);
    safe fn foo() -> Foo;
}

pub fn example() {
    let mut a = foo();
    observe(&raw mut a);
    let mut b = a;
    observe(&raw mut b);
}
```

<details>

<summary>MIR</summary>

```
fn example() -> () {
    let mut _0: ();
    let mut _1: Foo;
    let _2: ();
    let mut _3: *mut Foo;
    let _5: ();
    let mut _6: *mut Foo;
    scope 1 {
        debug a => _1;
        let mut _4: Foo;
        scope 2 {
            debug b => _4;
        }
    }

    bb0: {
        StorageLive(_1);
        _1 = foo() -> [return: bb1, unwind unreachable];
    }

    bb1: {
        StorageLive(_3);
        _3 = &raw mut _1;
        _2 = observe(move _3) -> [return: bb2, unwind unreachable];
    }

    bb2: {
        StorageDead(_3);
        StorageLive(_4);
        _4 = move _1;
        StorageLive(_6);
        _6 = &raw mut _4;
        _5 = observe(move _6) -> [return: bb3, unwind unreachable];
    }

    bb3: {
        StorageDead(_6);
        StorageDead(_4);
        StorageDead(_1);
        return;
    }
}
```

</details>

To understand why rustc is unable to perform move optimization, we need to look at the generated MIR in detail. In this example we would like the move from `a` to `b` to be eliminated. This is only possible if `a` and `b` have the same address, which would turn the assignment into a no-op.

`a` and `b` are mapped to locals `_1` and `_4` respectively in the MIR. Each local corresponds to a stack allocation with a certain lifetime. The lifetime of `_4` is specified by a pair of `StorageLive`/`StorageDead` statements, while `_1` has no such statements and its lifetime therefore spans the entire function. Since the lifetimes of the locals overlap, they are forbidden from having the same address.

There are 2 important factors at play here:

* MIR generation assigns storage lifetimes based on scope, meaning that the storage of `a` and `b` start from the `let` binding and end once the name goes out of scope.

* The assignment `_4 = move _1` is treated the same way as `_4 = copy _1` for operational semantics purposes: `move` is only used for borrow checking[^1].

[^1]: This is only true for assignments. `move` has special operational semantics for call arguments.

As a consequence, it is perfectly valid today from an operational semantics point of view for the first call to `observe` to stash a copy of the address of `a` and for the second call to then mutate the data behind that pointer.

Fundamentally, the language needs to be changed to allow `a` and `b` to share the same address in this example and eliminate the copy. Specifically, accessing memory that has been moved must become UB.

## Proposed language changes

This section describes the surface language changes to enable move optimization.

### Local allocation lifetimes

Currently, the lifetime of a local variable in a function starts from the point where it is defined (usually a `let` statement) and ends when execution exits its scope[^0]. During this lifetime, a variable has an associated *allocation* and address, which prevents any other allocation from having the same address. This lifetime is purely scope-based and independent of control flow.

[^0]: To be clear, this is what the compiler does today. The language spec is entirely silent on this matter.

This RFC proposes to instead only have the allocations of variables be live while they are *initialized*. This means that the underlying memory for variables is:
- allocated at the point where it is initialized (instead of where it is declared).
- freed when a variable of a non-`Copy` type is *moved* (or at the end of its scope, whichever is first).

This notably means that some existing code which may have been valid according to Miri would now be UB: accessing (read or write) a variable after it has been moved is no longer allowed since the allocation has been freed.

```rust
struct Foo(i32); // Doesn't implement Copy

fn main() {
    let a = Foo(123);
    let ptr = &raw const a;
    drop(a);

    // This is fine with the MIR/LLVM IR we
    // generate today: the storage of `a`
    // is valid until the end of the scope.
    //
    // With this RFC, this read would be UB.
    let b = unsafe { ptr.read() };
    assert_eq!(b.0, 123);
}
```

### Addresses and re-initialization

The definition above leads to a surprising behavior: if a variable is moved and later re-initialized, it will receive a new allocation which may have a different address than it previously had.

```rust
struct Foo(i32); // Doesn't implement Copy

fn main() {
    let mut a = Foo(123);
    let ptr1 = &raw const a;
    drop(a);
    a = Foo(123);
    let ptr2 = &raw const a;

    // This assert always succeeds today.
    // With this RFC, it may fail.
    assert_eq!(ptr1, ptr2);
}
```

Unfortunately, adding the guarantee that a variable keeps its original address when re-initialized introduces complications in the operational semantics if we also want to make that address available for other allocations while it is uninitialized.

Consider what happens if another local `b` needs to be allocated while `a` is uninitialized: to determine whether it can reuse the address of `a`, the Rust AM would need to predict the future to see whether the lifetimes of `a` and `b` ever overlap.

While it is possible to specify the operational semantics in terms of "no-behavior" (NB) when selecting the address of a variable, this makes reasoning about program execution very complex, and such reasoning is necessary to justify compiler optimizations.

<details>

<summary>Why NB can be a problem</summary>

With NB semantics we could specify that, starting from the non-deterministic choice of selecting an address for an allocation, any choice that leads to 2 allocations overlapping at the same time has no behavior. The program is well-defined if at least one choice does not result in NB, and the AM is required to make choices that do not result in NB in the future. This is in contrast to UB where if *any* choice can lead to UB then the whole execution is invalid.

NB can lead to surprising "time-traveling" behavior, especially when UB and NB are mixed together. For example:

```rust
// This program has no UB
let x = String::new();
let xaddr = &raw const x;
let y = x; // Move out of x and de-initialize it.
let yaddr = &raw const y;
x = String::new(); // assuming this does not change the address of x
// x and y are both live here. Therefore, they can't have the same address.
assume(xaddr != yaddr);
drop(x);
drop(y);
```

```rust
// This program has UB
let x = String::new();
let xaddr = &raw const x;
let y = x; // Move out of x and de-initialize it.
let yaddr = &raw const y;
// So far, there has been no constraint that would force the addresses to be different.
// Therefore we can demonically choose them to be the same. Therefore, this is UB.
assume(xaddr != yaddr);
// If the addresses are the same, this next line triggers NB. But actually this next
// line is unreachable in that case because we already got UB above...
x = String::new();
// x and y are both live here.
drop(x);
drop(y);
```

</details>

### Partial moves

The proposed behavior of freeing a local variable's allocation on move only applies when the entire variable is moved. This RFC leaves the exact behavior of partial moves (e.g. only one field of a struct) unspecified.

There are 2 reasons for this:

1. Specifying the behavior of partial moves makes the opsem (and by extension Miri) much more complex since we would now need to track which bytes of a local have been moved out and become "inactive" (UB to access). What happens to the padding of a struct if one field is moved out? What happens to the discriminant of an enum like `Option<T>` if the `Some` value has been moved out and the layout is optimized (the discriminant occupied the same bytes as the value)? These are all questions that would have to be answered.

2. The proposed MIR optimization pass can't easily take advantage of this, and even if it could (while respecting address observation rules) then the expected benefit over the existing proposed pass would be minimal. It's just not worth the extra complexity of tracking lifetimes separately for every field of a local.

## Proposed MIR changes

This section describes the changes to the MIR representation that are needed to implement the proposed language semantics while also enabling the actual move optimizations.

### Storage lifetime

Currently, the lifetime of locals in a function is determined using `StorageLive` and `StorageDead` statements in MIR. These serve two purposes:
- They are lowered to [`llvm.lifetime.start`](https://llvm.org/docs/LangRef.html#int-lifestart) and [`llvm.lifetime.end`](https://llvm.org/docs/LangRef.html#llvm-lifetime-end-intrinsic) intrinsics which are used by LLVM for stack slot coloring, which reduces stack usage.
- `StorageDead` is also used by the borrow checker to ensure that any borrows do not outlive the underlying allocation.

This RFC proposes to change MIR to make the lifetime of a local implicitly defined as the point where it is initialized to the point where its contents are moved out. This involves reworking the semantics of `StorageLive` and `StorageDead`. This has previously been proposed in [this issue](https://github.com/rust-lang/rust/issues/68622), but the idea is further expanded here.

The `StorageLive` and `StorageDead` are still kept as separate statements in MIR for 2 reasons:
1. They indicate where codegen should insert LLVM lifetime intrinsics.
2. `StorageDead` marks the end of the scope in which a local is defined, which ends any borrows of that local.

The precise semantics of `StorageLive` and `StorageDead` are re-defined in a section [below](#precise-semantics-for-local-lifetimes).

#### Initialization

`StorageLive` no longer allocates the underlying memory for a local. Instead, any MIR statement or terminator which writes to a place that has no `Deref` projections[^2] will implicitly allocate the storage for that local[^3] before writing to it. This has no effect if the storage for that local is already allocated[^4].

Writes to places that *do* have a `Deref` projection will still require that the base local be allocated, otherwise behavior is undefined.

[^2]: This is similar, but not identical to the concept of [move paths](https://rustc-dev-guide.rust-lang.org/borrow_check/moves_and_initialization/move_paths.html) used by the borrow checker to track which parts of a local are currently initialized.
[^3]: If the local was previously freed by `StorageDead` or a move, this new allocation may have a different address than the previous one.
[^4]: This is intentionally different from the old behavior of `StorageLive` which will free the old allocation and create a new one if a local is already allocated. This new behavior is necessary to correctly handle control flows where a local is only allocated in one branch but not the other.

#### De-initialization

The effects of `StorageDead` are now implicitly performed when a local is moved as a MIR operand. This applies uniformly to any MIR statement or terminator that takes a `move` operand, not just assignments. This will de-allocate the storage for the local, allowing its address to be re-used by a later allocation. Any use of the local, even taking its address, is UB if the local is unallocated.

The separate `StorageDead` statement is still necessary to mark the end of the scope in which a local is defined. However, it has no effect if the local has already been freed.

`move` operands only have the effect of de-allocating the storage of a local when used with a bare, unprojected local. If the local has projections then `move` behaves identically to `copy`.

### MIR evaluation order

Since `move` operands and writes to destination places now effectively have allocation/deallocation side-effects, it is necessary to precisely define the order in which the side-effects of a MIR statement or terminator occur. The general rule is that operands are evaluated left to right, except for destination places which are always evaluated last. This ordering applies uniformly to every kind of MIR statement and terminator, not just plain assignments. This differs from the current evaluation order in Miri and MiniRust which evaluates the destination place first.

The new ordering means that in a MIR assignment with a move like `_1 = move _2`, the source place is deallocated before the destination place is allocated. Since the two places are never allocated at the same time, they are allowed to share the same address.

One consequence of this is that it is illegal for the same local to be moved multiple times in the same MIR statement since it will become deallocated after the first move is evaluated.

Changing the evaluation order is not a breaking change for surface Rust because we can still control the final evaluation order during MIR construction.

### Precise semantics for local lifetimes

In this new model, `StorageLive` and `StorageDead` no longer directly allocate memory for a local and instead only serve as an indicator of where to insert LLVM lifetime intrinsics. However this means that we still need to specify their behavior in MIR to determine where it is valid to place them. To do this we add a phantom "live" state, in which a local exists but does not yet have an allocation. Locals are always in one of three states: **dead**, **live**, and **allocated**.

Now, we need to clarify the behavior of evaluating places in MIR[^place-evaluation]. The base local of a place is evaluated in one of two modes depending on its syntactic context:
- **Destination place mode**: the place appears as the destination of a MIR statement or terminator and does not contain any `Deref` projection. Evaluating the base local in this mode allocates it if it is currently **live**.
- **Non-destination place mode**: every other place, including destination places that contain a `Deref` projection. Evaluating the base local in this mode is UB unless it is already **allocated**.

[^place-evaluation]: Place evaluation in MIR either produces a pointer inside a valid allocation, or results in UB.

State transitions are defined as follows:

| Use of local | Precondition | Postcondition |
|---|---|---|
| Function entry (arguments) | N/A | State starts as **allocated** |
| Function entry (return place) | N/A | State starts as **live**[^ret] |
| Function entry (other locals) | N/A | State starts as **dead** |
| `StorageLive` | None | State becomes **live**[^live] |
| `StorageDead` | None | State becomes **dead** |
| Evaluation of a base local in destination place mode | UB if **dead** | State becomes **allocated** |
| Evaluation of a base local in non-destination place mode | UB if not **allocated** | State stays **allocated** |
| Evaluation of a move operand with no projections | UB if not **allocated** | State becomes **live**[^live2] |

The goal of these semantics is to ensure that the regions between `StorageLive` and `StorageDead` form an overapproximation of the actual time the variable is allocated and can hold data.

A prototype [MiniRust branch](https://github.com/Amanieu/minirust/compare/mir-move-elimination) implements the new semantics and shows how they interact with the rest of the language specification.

[^ret]: We want the return place to start without an allocation so that it can potentially be merged with a local whose live range doesn't overlap. It starts as **live** instead of **dead** because LLVM lifetime intrinsics don't work on the return place anyways, so there's no point emitting `StorageLive`/`StorageDead` for it.

[^live]: If the state was previously *live*, then any previous allocation is lost and a new one will be created when the local is later re-initialized. This matches the LLVM semantics of `llvm.lifetime.start` which will reset an allocation to `undef` if it is already live.

[^live2]: This frees the allocation since **live** doesn't have an allocation, only **allocated** does. However it can be re-initialized without the need for another `StorageLive`.

### MIR call terminators

MIR currently treats `copy` and `move` operands identically (meaning a `move` is the same as a `copy`) except in the context of function arguments in a `Call` terminator[^5]. There, `move` has a special meaning: rather than copying the operand value to a new place in the callee, the place is temporarily "donated" to the callee so that the corresponding argument local in the callee may have the same address as the argument place in the caller[^6]. It is non-deterministic whether the argument in the callee re-uses the existing place or uses a new allocation, and in practice this depends on whether the calling convention passes the argument by value or by reference (which is only known post-monomorphization).

[^5]: [rust-lang/rust#71117](https://github.com/rust-lang/rust/issues/71117)
[^6]: The exact behavior is still an open question today (which this RFC specifies), but this describes what codegen currently does.

This RFC doesn't change this special meaning of `move` operands in call terminators. Unlike other statements and terminators, move operands of a whole local in call terminators do *not* cause the local to be immediately deallocated. Instead this freeing is deferred until after the call has returned, when the callee has finished using the place.

## MIR move optimization

Rustc currently has a [destination propagation](https://github.com/rust-lang/rust/blob/master/compiler/rustc_mir_transform/src/dest_prop.rs) MIR optimization that attempts to eliminate unnecessary copies between locals. However, it is currently limited to copies between unprojected locals and only supports locals whose addresses are not taken.

This RFC proposes a new optimization pass which replaces the existing destination propagation pass. A [prototype](https://github.com/rust-lang/rust/pull/156046) of this optimization has been implemented. Compiling rustc with itself using the new pass produces a binary that is 2-5% faster on debug/check perf benchmarks.

The following sections describe the different phases of this MIR pass.

### Phase 1: Liveness computation

A dataflow analysis is run to produce a `SparseIntervalMatrix` recording, for each local, the set of points in the function at which it must have a distinct allocation. Two locals may have the same address if and only if their rows in this matrix are disjoint.

To model the fact that a source operand and a destination place in the same MIR statement may share an address (the source is read before the destination is written), liveness is evaluated at two points per statement: an **early** point at which source operands are read, and a **late** point at which destination places are written. Ranges in the matrix are recorded against these split points, so a local moved out at the early point of a statement does not appear as overlapping with a local written at the late point of the same statement.

A local's live range starts and ends according to the following rules:

- **Starts** at the late point of any statement or terminator that writes to it with a place that has no `Deref` projection.
- **Ends** at the early point of any statement or terminator that:
    - is a `StorageDead` for the local.
    - is a `Drop` of the whole local.
    - contains a move operand of the whole local (operand `move _x` with no projections).
    - is the last use of the local on this control-flow path, as long as the address of this local is never observed.

`Call` terminators are handled specially: `move` operands of a `Call` are kept live through the late point of the terminator, so they conflict with each other and with the destination place. This matches the runtime behavior of [MIR call terminators](#mir-call-terminators), where the place is donated to the callee for the duration of the call.

### Phase 2: Local unification

This phase produces a mapping from each unifiable local to the place it should be replaced with. Once the mapping is complete, a single rewrite pass over the body replaces every use of each remapped local with its mapped place.

Even though the liveness matrix would let us merge any two locals whose live ranges are disjoint, merging arbitrary unrelated locals is not useful: it is already handled by LLVM. The point of this pass is instead to *eliminate copies*, so the only candidates considered are pairs of locals that appear on opposite sides of an assignment. Merging such a pair turns the assignment into a self-assignment that can be deleted, which is the actual win.

Concretely, a mapping from `_2` to `_1` is added when a MIR assignment of the form `_1 = move _2` or `_1 = copy _2` is found where the live ranges of `_1` and `_2` in the `SparseIntervalMatrix` are disjoint. The live ranges of the two locals are then unioned in the matrix so that further unification decisions account for the merged lifetime. The original assignment (now `_1 = move _1`) can be eliminated as a no-op.

A local can also be mapped to a *projection* of another local. For MIR assignments of the form `_1 = Aggregate(move _2, move _3, ..)`, each field operand is considered for unification with the corresponding field of the destination: this produces mappings from `_2` to `_1.0`, from `_3` to `_1.1`, and so on. After rewriting, the copies for unified fields become no-ops since the source and destination of each field have the same address.

### Phase 3: Storage reconstruction

The original `StorageLive`/`StorageDead` statements no longer reflect the merged liveness produced by unification, so they are all removed and rebuilt from the liveness matrix. New `StorageLive`/`StorageDead` statements are inserted based on the (now-unified) liveness matrix. For each local, each live interval in the matrix produces:
- A `StorageLive` at the point where the live range starts. If the range starts at the entry of a block, the `StorageLive` is hoisted into every predecessor in which the local is dead at the terminator. In some cases this requires splitting critical edges so that the storage statement does not extend the live range onto a sibling branch.
- A `StorageDead` at the point where the live range ends. If the range ends at a terminator, the `StorageDead` is emitted at the start of each successor in which the local is dead on entry. Critical edges do not need to be split in this direction because an extra `StorageDead` on a path where the local is already dead has no effects.

### Phase 4: Aliasing assignment fixup

MIR assignments currently have the constraint that, for types which are not treated as scalars in codegen, the source and destination places are [not allowed to overlap](https://github.com/rust-lang/rust/issues/68364). This constraint exists to make codegen easier since such assignments can be lowered to a `memcpy` call, which would result in incorrect code for assignments like `_1 = (_1.1, _1.0)`.

After local unification some MIR assignments may end up with overlapping source and destination places. A final pass walks the MIR and rewrites any such assignments to restore the no-aliasing invariant. For each assignment:
- Self-assignments (e.g. `_1 = _1`) are deleted outright.
- Simple `Use` assignments where source and destination overlap but are not identical are split: the source is read into a fresh temporary first, and the temporary is then moved into the destination.
- Aggregate assignments where any operand aliases the destination are decomposed into per-field assignments. Per-field self-assignments (where source and destination resolve to the same place) are deleted outright. Other aliasing fields are hoisted into temporaries first, then all destination fields are written in a second pass. For enum aggregates the discriminant is set after fields are written.
- Other rvalues (e.g. `Repeat`, `Cast`) are hoisted whole into a temporary if any of their operands alias the destination.
- Rvalues that operate only on scalar types (binary/unary ops, `Discriminant`, `Ref`, `RawPtr`, etc.) are left untouched because their codegen does not rely on the no-aliasing assumption.

## Drawbacks

### Impact on MIR optimizations

MIR optimizations need to be careful not to shorten the live range of a local whose address has been taken by moving or eliminating assignments or moves. Doing so could cause the lifetime analysis to conclude that 2 locals could share the same address when this would not be allowed in the source program. Extending the live range of a local is not a problem since it just pessimizes the optimization by forcing locals to have separate addresses.

In practice the only MIR optimization that can shorten the lifetime of a local whose address has been taken is `DeadStoreElimination`: all other optimizations only touch locals whose address is never taken, and are therefore unaffected by this change. This is particularly relevant because `DeadStoreElimination` runs before the move optimization in the MIR pipeline, so any dead stores it removes are gone by the time the liveness analysis in this pass runs.

`DeadStoreElimination` will need to be adjusted to preserve dead stores rather than replacing them with debuginfo statements, so that the liveness analysis still sees them. This could be done by introducing a new `StatementKind::DeadAssign` which the liveness analysis treats as a write but which codegen ignores.

For example, consider the following code:

```rust
struct Foo([u8; 100]);

unsafe extern "C" {
    safe fn observe(b: *mut Foo);
    safe fn foo() -> Foo;
}

pub fn example() {
    let mut a;
    let mut b;

    b = foo(); // dead store

    a = foo();
    observe(&raw mut a);
    b = a;
    observe(&raw mut b);
}
```

In this example, `a` and `b` are live at the same time, but eliminating the dead store would allow them to share the same address, which is not allowed by the original program.

### Potentially breaking change

This is technically a breaking change since it effectively reduces the live range for which an allocation is valid. This has 3 effects in practice, the most obvious one being that it's possible to write unsafe code that was previously accepted by Miri but that will result in UB under this new model. For example:

```rust
struct Foo(i32); // Doesn't implement Copy

fn main() {
    let a = Foo(123);
    let ptr = &raw const a;
    drop(a);

    // This is fine today: the storage of `a`
    // is valid until the end of the scope.
    let b = unsafe { ptr.read() };
    assert_eq!(b.0, 123);
}
```

Such examples are almost always contrived and unlikely to occur in real code. In fact, the behavior of allocations after a move is an [open question](https://github.com/rust-lang/unsafe-code-guidelines/issues/188) in the unsafe code guidelines and not something that we make a hard guarantee on in the language.

The second effect is that users can observe the address of a local changing when it is moved and later re-initialized:

```rust
struct Foo(i32); // Doesn't implement Copy

fn main() {
    let mut a = Foo(123);
    let ptr1 = &raw const a;
    drop(a);
    a = Foo(123);
    let ptr2 = &raw const a;

    // This assert always succeeds today.
    // With this RFC, it may fail.
    assert_eq!(ptr1, ptr2);
}
```

The last effect of this change is that users will now be able to observe that some pairs of places share an address when this would previously have been impossible. The example from earlier demonstrates this:

```rust
struct Foo([u8; 100]);

unsafe extern "C" {
    safe fn observe(b: *mut Foo);
    safe fn foo() -> Foo;
}

pub fn example() {
    let mut a = foo();
    observe(&raw mut a);
    let mut b = a;
    observe(&raw mut b);
}
```

In some situations, users may rely on the address of a local as a unique hash map key. But even then it's unreasonable to assume that this address remains unique once the local is dropped, so breakage in practice should be non-existent.

## Rationale and alternatives

### Preserving addresses on re-initialization

One of the changes made by this RFC is that named local variables may change their address if they are de-initialized and then later re-initialized. The reason for this change is to avoid relying on NB semantics for opsem. However there is another possibility which avoids NB, which is to pre-compute the lifetimes of all locals in pre-optimization MIR and then encode in MIR which locals have overlapping lifetimes and are thus not allowed to be merged together. MIR optimizations would then rely on this data to determine whether two locals can be merged.

The problem with this approach is that the overlap constraints are hard-coded into the MIR and are unable to evolve as optimizations perform constant propagation and eliminate unreachable branches. Specifying lifetimes directly in the opsem is preferable and results in much cleaner semantics for optimizations to work with.

### Performing (de)allocation in bulk instead of during place/operand evaluation

This RFC defines the allocation of locals as a side effect of evaluating destination places and `move` operands. An alternative is to instead make this an explicit step of the enclosing MIR statement or terminator, performed between operand evaluation and destination place evaluation. Under this model, the semantics of an assignment would be:

1. Evaluate the input operands.
2. Deallocate any locals used as bare `move` operands.
3. Allocate the base local of the destination place (if no `Deref`).
4. Evaluate the destination place.
5. Store the result of the statement/terminator into the destination place.

The advantage of this framing is that place and value expression evaluation has no side-effects, which makes it easier to reason about the order in which subexpressions can be evaluated. The disadvantage is that statement evaluation semantically needs to iterate through operands twice.

MIR building, MIR optimizations and codegen can be adapted to work with either model, the choice between them is largely arbitrary.

## Unresolved questions

None

## Future possibilities

### Applying this optimization to `Copy` types

One surprising consequence of these changes is that implementing `Copy` for a type can inhibit the move optimization. This is because a copy doesn't invalidate any borrows of a value and therefore forbids re-using the allocation. Consider the following example:

```rust
let mut a = 1; // i32 implements Copy

// This may save the address of a to access later.
observe(&raw mut a);

// This is a copy, it does *not* invalidate the borrow of a.
let mut b = a;

// This is allowed to access *both* a and b, so merging these is not allowed.
observe(&raw mut b);
```

It might be possible to address this with a language-level `move` keyword which forces a move even for `Copy` types, but at this point it's not clear that there is sufficient justification for adding this. Here's an example of how this keyword would work:

```rust
let x = 1;
let y = &x;
drop(move x);
let z = *y; // Fails because x was moved. Removing `move` fixes this.
```
