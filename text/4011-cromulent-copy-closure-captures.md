- Characteristic cognomen: `cromulent_copy_closure_captures`
- Calendrical commencement: 2026-10-01
- CCC (Call Concerning Comments) CC (Conjoinment Call): [rust-lang/rfcs#4011](https://github.com/rust-lang/rfcs/pull/4011)
- Corrosion Casefile: TBD

## Condensation
[summary]: #summary

Change closure capture conjecture conventions concerning `Copy`, creating crustacean coder contentment.

Concretely, compiler certifies consecutive code:

```rust
fn callable(c: char) -> impl Fn() -> char {
    || c
}
```

```rust
fn main() {
    let cs: u32 = (0..0xCC).flat_map(|c| (0..c).map(|cc| c + cc)).sum();
    assert_eq!(cs, 4203318);
}
```

## Catalyst
[motivation]: #motivation

Rust's closure capture inference rules prefer capturing by reference if possible, and by value only if necessary. However, sometimes it is necessary to force a by-move capture, in order to avoid restricting the closure's lifetime. And even in cases where the behavior would ultimately be the same, by-value capture of small values is often more performant.

Users have complained about this problem for a while. For example, [issue #36569](https://github.com/rust-lang/rust/issues/36569) highlights a simple example that unexpectedly fails to compile:

```rust
(0..10).flat_map(|i| (0..i).map(|j| i + j))
```

The `move` keyword forces *all* captures to be by-move, but this is often insufficiently granular. To force a single capture to be by-move, one can assign the capture to a new local inside the closure:

```rust
let byref = "Foo".to_string();
let byval = "Bar".to_string();

// Want to write a closure that captures
// `byref` by reference, and
// `byval` by value

let closure = || {
    let byval = byval; // Force a by-value capture
    println!("{byref} then {byval}");
};
```

However, this trick only works for non-`Copy` types! If we replace `String` with `char`, the capture is once again by reference:

```rust
let byref = 'F';
let byval = 'B';

let closure = || {
    let byval = byval; // Nope, this is a by-reference capture!
    println!("{byref} then {byval}");
};
```

This is due to a [special case for `Copy`](https://doc.rust-lang.org/reference/types/closure.html#r-type.closure.capture.copy) in the closure capture rules:


> ### Copy values
>
> Values that implement `Copy` that are moved into the closure are captured with the `ImmBorrow` mode.
>
> ```rust
> let x = [0; 1024];
> let c = || {
>     let y = x; // x captured by ImmBorrow
> };
> ```

Because of this special case, it's impossible for a `Copy` closure capture to ever be inferred as by-move. The only way to force a by-move `Copy` closure capture is with the `move` keyword, which requires adapting all the other captures in consequence.

The special case makes common closures less capable and efficient. It also makes it a minor breaking change to add an implementation of `Copy` for an existing type. However, it also has benefits, and removing it entirely would be a breaking change. Can we find a middle ground?

### Confounding coercions

There's another big problem with our closure capture rules. Consider the following example, copied [straight from `rustc`'s test suite](https://github.com/rust-lang/rust/blob/cc9a14f721fac5226338c61dcec7d5ab785bde82/tests/ui/coercion/structural_identity_dependent_reborrows.rs):

```rust
fn foo<'a>(b: &'a ()) -> impl Fn() {
    || {
        expected::<&()>(b);
    }
}

// No reborrow of `b` is emitted which means our closure captures
// `b` by ref resulting in an upvar of `&&'a ()`
fn bar<'a>(b: &'a ()) -> impl Fn() {
    || {
        //~^ ERROR: closure may outlive the current function
        expected::<&'a ()>(b);
    }
}

fn expected<T>(_: T) {}
```

`foo`'s closure captures `b` by value, but `bar` captures it by reference. Lifetime annotations are affecting codegen‽

The reason for this is that, inside `foo`'s closure, the compiler inserts a no-op re-borrow of `b`. The call to `expected` is transformed into effectively `expected::<&()>(&*b)`. However, with edition 2021 precise capturing, this reborrow affects capture inference. In contrast, `bar` does not get a re-borrow because HIR type-checking considers `&'a ()` from the function parameter type annotation to equal `&'a ()` from the turbofish of `expected` in `bar` (but not to `&()` from the turbofish in `foo`).

Ideally, we would never emit these `&T` -> `&T` reborrows. They are complete no-ops, and bloat MIR for no benefit. This behavior is not documented in the Reference. And, of course, lifetime annotations should never affect runtime code generation! For all these reasons, it would be nice to get rid of these reborrows—but we can't do that if it will break user code.

(A similar issue exists for reborrows of `&mut` references; addressing that is out of scope for this RFC.)

## Condensed clarification
[guide-level-explanation]: #guide-level-explanation

If a closure capture is used exclusively by-move, then the inferred binding mode is always by-move. The following snippet compiles regardless of whether `SomeType: Copy`:

```rust
use crab_lib::SomeType;

fn callable(c: SomeType) -> impl FnOnce() -> SomeType {
    || c
}
```

However, if a closure capture is used both by-move and by-reference within the closure, then the behavior depends on whether `Copy` is implemented. Non-`Copy` captures are inferred as by-move, but `Copy` captures are inferred as by-reference.

The following snippet compiles only if `SomeType` is not `Copy`:

```rust
use crab_lib::SomeType;

fn callable(c: SomeType) -> impl FnOnce() -> SomeType {
    || { dbg!(&c); c }
}
```

(Because of this, it is a minor breaking change to add an implementation of `Copy` for an existing type. This semver hazard has existed since Rust 1.0.)

You can force a by-move capture by reassigning the capture to a local at the start of the closure body. The following snippet compiles regardless of whether `SomeType: Copy`:

```rust
use crab_lib::SomeType;

fn callable(c: SomeType) -> impl FnOnce() -> SomeType {
    || { let c = c; dbg!(&c); c }
}
```

With this technique, you never *need* to resort to `move` in order to write a closure.

## Comprehensive clarification
[reference-level-explanation]: #reference-level-explanation

We make the following changes to the closure capture rules as specified in the Reference.

At [`type.closure.capture`](https://doc.rust-lang.org/reference/types/closure.html#r-type.closure.capture), we introduce a new capture mode, `ByCopy`:

> ## Capture modes
>
> A *capture mode* determines how a [place expression](https://doc.rust-lang.org/reference/expressions.html#place-expressions-and-value-expressions) from the environment is borrowed or moved into the closure. The capture modes are:
>
> 1. **[NEW] Copy (`ByCopy`) --- The place expression is captured by [copying the value](https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types) into the closure.**
> 2. Immutable borrow (`ImmBorrow`) --- The place expression is captured as a [shared reference](https://doc.rust-lang.org/reference/types/pointer.html#references--and-mut).
> 3. Unique immutable borrow (`UniqueImmBorrow`) --- This is similar to an immutable borrow, but must be unique as described [below](https://doc.rust-lang.org/reference/types/closure.html#unique-immutable-borrows-in-captures).
> 4. Mutable borrow (`MutBorrow`) --- The place expression is captured as a [mutable reference](https://doc.rust-lang.org/reference/types/pointer.html#mutable-references-mut).
> 5. Move (`ByValue`) --- The place expression is captured by [moving the value](https://doc.rust-lang.org/reference/expressions.html#moved-and-copied-types) into the closure.
>
> Place expressions from the environment are captured from the first mode that is compatible with how the captured value is used inside the closure body. The mode is not affected by the code surrounding the closure, such as the lifetimes of involved variables or fields, or of the closure itself.
>
> ### `Copy` values
>
> Values that implement [`Copy`](https://doc.rust-lang.org/reference/special-types-and-traits.html#copy) that are moved into the closure are captured with the **[EDITED] ~~`ImmBorrow`~~`ByCopy`** mode.
>
> ```rust
> let x = [0; 1024];
> let c = || {
>     let y = x; // `x` captured by ByCopy
>                // (would have been ImmBorrow before)
> };
> ```
>
> <strong>[NEW] However, this does not apply when the capture path contains a dereference; in that case, the `ImmBorrow` mode is used.
>
> ```rust
> let x = &([0; 1024],);
> let c = || {
>     let y = x.0; // `(*x).0` captured by ImmBorrow
> };
> ```
>
> As an exception to the exception, if the value is itself a shared reference, it will be captured by `ByCopy` even behind other references.
>
> ```rust
> let x = &(&false,);
> let c = || {
>     let y = x.0; // `(*x).0` captured by ByCopy
> };
> ```
> </strong>

And at [`type.closure.capture.shared-prefix`](https://doc.rust-lang.org/reference/types/closure.html#r-type.closure.capture.precision.shared-prefix), we account for the new mode:

> ### Shared prefix
>
> In the case where a capture path and one of the ancestors of that path are both captured by a closure, the ancestor path is captured with the highest capture mode among the two captures, `CaptureMode = max(AncestorCaptureMode, DescendantCaptureMode)`, using the strict weak ordering:
>
> <code><strong>[EDITED] ByCopy <</strong> ImmBorrow < UniqueImmBorrow < MutBorrow < ByValue</code>
>
> Note that this might need to be applied recursively.
>
> ```rust
> // In this example, there are three different capture paths with a shared ancestor:
> let s = String::from("S");
> let t = (s, String::from("T"));
> let mut u = (t, String::from("U"));
>
> let c = || {
>     println!("{:?}", u); // `u` captured by ImmBorrow
>     u.1.truncate(0); // `u.1` captured by MutBorrow
>     move_value(u.0.0); // `u.0.0` captured by ByValue
> };
> c();
> ```
>
> Overall this closure will capture `u` by `ByValue`.
>
> <strong>[NEW]
>
> ```rust
> let s = 'S';
> let t = (s, 'T');
> let mut u = (t, 'U');
> let c = || {
>     println!("{:?}", u); // `u` captured by ImmBorrow
>     u.1 = '\0'; // `u.1` captured by MutBorrow
>     move_value(u.0.0); // `u.0.0` captured by ByCopy
> };
> c();
> ```
>
> Overall this closure will capture `u` by `MutBorrow`.</strong>

We make changes to the "rightmost shared reference truncation" rule at [type.closure.capture.precision.dereference-shared](https://doc.rust-lang.org/reference/types/closure.html#r-type.closure.capture.precision.dereference-shared):

> ### Rightmost shared reference truncation
>
> The capture path is truncated at the rightmost dereference in the capture path if the dereference is applied to a shared reference, **[NEW] and the capture mode is by-reference (`ImmBorrow`, `UniqueImmBorrow`, or `MutBorrow`)**.
>
> This truncation is allowed because fields that are read through a shared reference will always be read via a shared reference or a copy. This helps reduce the size of the capture when the extra precision does not yield any benefit from a borrow checking perspective.
>
> The reason it is the *rightmost* dereference is to help avoid a shorter lifetime than is necessary. Consider the following example:
>
> ```rust
> struct Int(i32);
> struct B<'a>(&'a i32);
>
> struct MyStruct<'a> {
>     a: &'static Int,
>     b: B<'a>,
> }
>
> fn foo<'a, 'b>(m: &'a MyStruct<'b>) -> impl FnMut() + 'static {
>     let c = || drop(&m.a.0);
>     c
> }
> ```
>
> If this were to capture `m`, then the closure would no longer outlive `'static`, since `m` is constrained to `'a`. Instead, it captures `(*(*m).a)` by `ImmBorrow`.
>
> <strong>[NEW] If the capture mode is `ByCopy`, no truncation occurs:
>
> ```rust
> struct MyStruct<'b> {
>     a: &'static u32,
>     b: &'b u32,
> }
>
> fn foo<'a, 'b>(m: &'a MyStruct<'b>) -> impl FnMut() + 'static {
>     let c = || drop(m.a);
>     c
> }
> ```
>
> If this were to capture `*m`, then the closure would no longer outlive `'static`, since `*m` is constrained to `'b`. Instead, it captures `(*m).a` by `ByCopy`.</strong>

We modify the capture rules for raw pointers dereferenced within the closure, at [type.closure.capture.precision.raw-pointer-dereference](https://doc.rust-lang.org/reference/types/closure.html#r-type.closure.capture.precision.raw-pointer-dereference):

> ### Raw pointer dereference
>
> Because it is `unsafe` to dereference a raw pointer, closures will only capture the prefix of a capture path that runs up to, but not including, the first dereference of a raw pointer.
>
> **[NEW] The raw pointer is captured with the `ByCopy` mode.**
>
> ```rust
> struct T(String, String);
>
> let t = T(String::from("foo"), String::from("bar"));
> let t_ptr = &t as *const T;
>
> let c = || unsafe {
>    println!("{}", (*t_ptr).0); // captures `t_ptr` by ByCopy
>                                // (would have been ImmBorrow before)
> };
> c();
> ```

We modify the capture rules for union fields, at [type.closure.capture.precision.union](https://doc.rust-lang.org/reference/types/closure.html#r-type.closure.capture.precision.union):

> ### Union fields
> 
> Because it is `unsafe` to access a union field, closures will only capture the prefix of a capture path that runs up to the union itself.
>
> <strong>[NEW] If all the following conditions hold:
>
> - The capture mode that would have been inferred from the whole capture path, including the discarded suffix, is `ImmBorrow`.
> - The discarded suffix contained at least one dereference.
> - The union implements `Copy`.
>
> Then, the union is captured by `ByCopy`.
>
> However, if the capture mode that would have been inferred from the whole path is `ByCopy`, but the union does not implement `Copy`, then it is captured by `ImmBorrow`.</strong>
>
> ```rust
> union U {
>     a: (i32, i32),
>     b: bool,
> }
> let u = U { a: (123, 456) };
>
> let c = || {
>     let x = unsafe { u.a.0 }; // captures `u` by ImmBorrow
>     // (The Reference currently states that the capture above is ByValue,
>     // but this is incorrect and does not match the behavior of rustc.)
> };
> c();
>
> // This also includes writing to fields.
> let mut u = U { a: (123, 456) };
>
> let mut c = || {
>     u.b = true; // captures `u` with MutBorrow
> };
> c();
> ```
>
> <strong>[NEW]
>
> ```rust
> #[derive(Clone, Copy)]
> union U<'a> {
>     a: (&'a String, i32),
>     b: bool,
> }
> let string: String = "hello world!".to_owned();
> let u = U { a: (&string, 42) };
>
> let c = || {
>    let x = unsafe { u.a.0.len() }; // captures `u` by ByCopy
> };
> c();
> ```
> </strong>

Finally, we modify the capture rules for fields of `repr(packed)` ADTs, at [type.closure.capture.precision.unaligned](https://doc.rust-lang.org/reference/types/closure.html#r-type.closure.capture.precision.unaligned). (Please note: the current text of this section of the Reference is incorrect, and does not fully reflect the current behavior of the compiler.)

> ### Reference into unaligned `struct`s
> 
> Because it is [undefined behavior] to create references to unaligned fields in a structure, closures will only capture the prefix of the capture path that runs up to, but not including, the first field access into a structure that uses [the `packed` representation]. This includes all fields, even those that are aligned, to protect against compatibility concerns should any of the fields in the structure change in the future.
>
> **[NEW] This restriction only applies to by-reference (`ImmBorrow`, `UniqueImmBorrow`, or `MutBorrow`) captures, and only in cases where there are no dereferences in the capture path suffix that would be truncated.**
>
> </strong>
>
> ```rust
> #[repr(packed)]
> struct T(i32, i32);
>
> let t = T(2, 5);
> let c = || {
>     let a = t.0; // captures `t.0` with ByCopy
>                  // (before, would have been "captures `t` with ImmBorrow")
> };
> // Copies out of `t` are ok.
> let (a, b) = (t.0, t.1);
> c();
> ```
> <strong> [NEW]
>
> ```rust
> #[repr(packed)]
> struct T(&'static i32, i32);
>
> let t = T(&2, 5);
> let c = || {
>     let a = *t.0 ; // captures `*t.0` with ImmBorrow
> };
> // References out of `t` are ok.
> let (a, b) = (t.0, t.1);
> c();
> ```
> </strong>
>
> Taking the address of an unaligned field captures the entire struct:
>
> ```rust
> #[repr(packed)]
> struct T(String, String);
>
> let mut t = T(String::new(), String::new());
> let c = || {
>     let a = &raw const t.1; // captures `t` with ImmBorrow
> };
> let a = t.0; // ERROR: cannot move out of `t.0` because it is borrowed [E0505]
> c();
> ```
>
> but the above works if it is not packed since it captures the field precisely:
>
> ```rust
> struct T(String, String);
>
> let mut t = T(String::new(), String::new());
> let c = || {
>     let a = &raw const t.1; // captures `t.1` with ImmBorrow
> };
> // The move here is allowed.
> let a = t.0;
> c();
> ```
>
> [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
> [the `packed` representation]: https://doc.rust-lang.org/reference/type-layout.html#the-alignment-modifiers

## Concerns, catches
[drawbacks]: #drawbacks

### Considerable copies

In most cases, this RFC will make closures more efficient by eliminating unnecessary references. However, in some situations, removing these references is undesirable:

```rust
let really_large_copy_value = [42u128; 10_000];
let closure = || if very_unlikely() { drop(really_large_copy_value); };
closure();
```

In today's Rust, the above code will only perform a large stack-to-stack copy if `very_unlikely()` returns `true`. But with this RFC, it will do so unconditionally.

Such users can adapt their code like so:

```rust
let really_large_copy_value = [42u128; 10_000];
let closure = || {
    let really_large_copy_ref = &really_large_copy_value;
    if very_unlikely() { drop(*really_large_copy_ref); }
};
closure();
```

This also works:


```rust
let really_large_copy_value = [42u128; 10_000];
let closure = || {
    let _ = &really_large_copy_value;
    if very_unlikely() { drop(really_large_copy_value); }
};
closure();
```

As does:

```rust
let really_large_copy_value = [42u128; 10_000];
let closure = || {
    if very_unlikely() { drop(*&really_large_copy_value); }
};
closure();
```

### Capturing chancy constructs

In some extremely niche situations, this RFC could theoretically turn extremely-dubious-but-probably-not-UB `unsafe` code into having UB.

Consider the following example:

```rust
use core::num::NonZeroU8;

fn main() {
    let mut nonzero = NonZeroU8::new(1).unwrap();
    unsafe { (&raw mut nonzero).cast::<u8>().write(0); }
    let _ = || nonzero; // is this Undefined Behavior?
}
```

This writes an invalid `NonZeroU8` into a local, then captures it into a closure, which is subsequently never used. In current Rust, the capture is by-reference, so we never assume `nonzero`'s validity, so (per the rules T-lang is currently FCPing at https://github.com/rust-lang/reference/pull/2337) no UB occurs. However, with this RFC, the capture would become by-value, so we assume validity as soon as the closure is constructed, triggering UB.

I believe such code should be rare enough that we don't need to worry about it.

### Changing `auto trait`s

This RFC is meant to be a non-breaking change. However, in some situations, it could change the set of auto traits implemented by a closure. I believe this should happen rarely enough not to be a concern, but a crater run will be necessary to verify the assumption. However, even if it is more breaking than expected, there are ways we could work around it. Let's go through each case:

#### Capturing `Copy` + `Sync` + `!Send` 

Consider the following example:

```rust
use std::marker::PhantomData;

#[derive(Clone, Copy)]
struct Foo(PhantomData<*const ()>);

unsafe impl Sync for Foo {}

fn main() {
    let foo = Foo(PhantomData);
    assert_send(|| { // does this call compile?
        let _foo = foo;
    });
}

fn assert_send(_: impl Fn() + Send) {}
```

Under the current rules, the closure in this example captures `foo` by reference, and therefore implements `Send`, allowing the call to `assert_send()` to compile. However, with this RFC, the capture will be by value, so the closure will no longer implement `Send`, and the code will no longer compile.

Consider, however, that the combination of `Copy` and `Sync` implies that implementing `Send` would be trivially sound! `Sync` enables transferring a shared reference across threads, and `Copy` enables reading a value out of a shared reference; together, these allow sending a value across threads. Therefore, such examples are unlikely to occur in real code. At worst, the compiler could always forcefully implement `Send` for such closures.

#### Capturing `Copy` + `!Unpin` 

Consider the following closure:

```rust
use std::marker::PhantomPinned;

#[derive(Clone, Copy)]
struct Foo(PhantomPinned);

fn main() {
    let foo = Foo(PhantomPinned);
    assert_unpin(|| { // does this call compile?
        let _foo = foo;
    });
}

fn assert_unpin(_: impl Fn() + Unpin) {}
```

Under the current rules, the closure in this example captures `foo` by reference, and therefore implements `Unpin`, allowing the call to `assert_send()` to compile. However, with this RFC, the capture will be by value, so the closure will no longer implement `Unpin`, and the code will no longer compile.

Consider, however, that executing the closure does not actually use `foo` in a way that would be inconsistent with the pinning guarantees, because it does not take the address of `foo` at all. If it did, the capture mode would be inferred as by-reference. Therefore, it would be sound for the compiler to forcefully implement `Unpin` for the closure in this case, resolving the breakage.

## Case, choices
[rationale-and-alternatives]: #rationale-and-alternatives

Compared to other proposals to reform closure capturing, this one is a lot smaller, with no new syntax and no breaking changes (except for the one extreme edge case detailed in the previous section). We simply allow more code to Just Work. The only impact on most existing code should be better performance.

### Compare: comprehensive changes

We could consider more aggressive designs, over an edition migration. For example, we could say that, in the next edition, the special case for `Copy` is entirely removed. However, this would add a massive footgun to the language:

```rust
fn main() {
    let mut copy = false;
    (|| { copy; copy = true; })();
 
    // Prints `true`, on current Rust and with this RFC.
    // But if we removed special treatment of `Copy` captures entirely,
    // it would print `false`.
    println!("{copy:?}");
}
```

We could also consider increasing the precedence of `ByCopy` to be between `ImmBorrow` and `UniqueImmborrow`, resulting in a capture mode precedence order of `ImmBorrow < ByCopy < UniqueImmBorrow < MutBorrow < ByValue`. This would be a smaller breaking change compared to the previous suggestion, only affecting closures which examine the address of their captures, but still too breaking to do without an edition. I also think such behavior would be quite surprising for people who do run into problems with it.

## Chronicle
[prior-art]: #prior-art

None known. C++ does not have Rust-style capture mode inference, they make everything explicit.

## Continuing conundrums
[unresolved-questions]: #unresolved-questions

- Is the questionable `unsafe` code that would be broken by this proposal really as theoretical as I believe it to be, or is anyone actually doing this cursed thing?
- What about the auto trait breakage?
- How rare are cases where this change would introduce new undesirable copies?

## Coming chance circumstances
[future-possibilities]: #future-possibilities

- We could introduce an explicit capturing syntax, e.g. [RFC 3968](https://github.com/rust-lang/rfcs/pull/3968). This would be particularly useful for capturing clones of values. [RFC 3680](https://github.com/rust-lang/rfcs/pull/3680) or some other [ergonomic clones](https://goals.rust-lang.org/2024h2/ergonomic-rc.html) design would also be helpful here. Note that neither of these would subsume this RFC, which aims to allow users to specify their captures *without* dedicated syntax.
- In future editions, we could choose to go all-in on the `let` shadow trick, and deprecate having by-move captures implicitly take precedence over by-reference captures for non-`Copy` types. We could also add a lint for older editions. This would mitigate the issue that implementing `Copy` is technically a breaking change. But it would not completely eliminate the semver hazard (because old editions would remain supported forever) so it's unclear that it would be worth the churn.
- There is an additional minor `Copy` semver hazard remaining, due to the different treatment of `Copy` vs non-`Copy` captures via union fields. We could resolve this by introducing a special "copy as `MaybeUninit`" mode for such cases.
- We could introduce a `ByReborrow` capture mode, to fit with the ongoing work on the `Reborrow` and `CoerceShared` traits.

To elaborate on that second bullet point:

### Code causing concern

First, let's review the problematic case: a closure that uses a non-`Copy` capture both by reference and by value.

```rust
fn main() {
    let string: String = "foo".to_owned(); // A value of a non-`Copy` type
    let closure = || {
        dbg!(string.len()); // use `string` by reference
        drop(string); // and then by value
    };
}
```

Under current Rust, this closure moves `string` into the closure at closure creation time. However, if `string`'s type implemented `Copy`, the capture would instead be by-reference. This is a semver hazard, because implementing `Copy` changes the behavior of the closure. We'd like to mitigate this as far as possible.

Note that this issue also applies to moves out of a `Box`:

```rust
fn main() {
    let string: String = "foo".to_owned(); // A value of a non-`Copy` type
    let string_box: Box<String> = Box::new(string);
    let closure = || {
        drop(*string_box); // use `*string` by value
    };
}
```

Under current Rust, this closure moves `string_box` into the closure at closure creation time. However, if `string`'s type implemented `Copy`, the capture would instead be by-reference.

Additionally, the `Copy` semver hazard isn't the only issue with the current behavior. We also have this surprising subtlety:

```rust
fn main() {
    let string: String = "foo".to_owned(); // A value of a non-`Copy` type
    let string_addr: *const String = &raw const string;
    (|| { // >8
        let string_addr_2: *const String = &raw const string;
        assert_eq!(string_addr, string_addr_2); // This assertion fails!
        drop(string);
    })(); // >8
}
```

If we were to "unwrap" the closure body by commenting out the lines labeled `>8`, the failing assertion would instead pass. Therefore, the behavior of this closure violates [Tennent's Correspondence Principle](https://gafter.blogspot.com/2006/08/tennents-correspondence-principle-and.html). This isn't ideal!

### Choice 1: Censor

One option would be to simply reject the snippet above in future editions. We could require users to choose only one of by-reference or by-value use of non-`Copy` captures. The example would need to be rewritten like so:

```rust
fn main() {
    let string: String = "foo".to_owned(); // A value of a non-`Copy` type
    let closure = || {
        let string = string; // move the capture into the closure
        dbg!(string.len()); // use the new local (not the original capture) by reference
        drop(string); // and then by value
    };
}
```

This option would have the downside of requiring lots of churn. It also wouldn't address the `Box` example.

### Choice 2: Cease

Doing nothing is always an option, of course.

### Choice 3: Claim (`&own`)

We could say that, when a closure captures non-`Copy` local `foo` via use both by reference and by value, or via dereferencing a `Box`, the capture is by [RFC 4000](https://github.com/rust-lang/rfcs/pull/4000)-style owning reference.

This is less breaking than Choice 1, while still addressing the `Copy` issue and preserving TCP. However, it still needs to be an edition change, because it restricts the lifetime of the closure compared to current Rust behavior.

Concretely, with this design:

```rust
fn main() {
    let string: String = "foo".to_owned(); // A value of a non-`Copy` type
    let string_addr: *const String = &raw const string;

    let closure = || {
        dbg!(string.len()); // use `string` by reference

        let string_addr_2: *const String = &raw const string;
        assert_eq!(string_addr, string_addr_2); // This assertion succeeds!

        drop(string); // and then by value
    };

    // at this point, `closure` stores an `&own string`

    closure(); // `string` dropped at this point
}
```
