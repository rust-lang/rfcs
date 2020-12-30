- Feature Name: `boxed_macro` 
- Start Date: 2020-12-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduces the `boxed` macro, which allows one to construct data directly on the heap. 
This provides an alternative method to box syntax that does not use "placement new" syntax. In addition, 
if box syntax is significantly revised in the future, these changes could be applied to the `boxed` macro to 
prevent ecosystem breakage.

# Motivation
[motivation]: #motivation

At the moment, the only safe way of constructing data directly on the heap in Rust is via box syntax.
The [`vec`](https://doc.rust-lang.org/src/alloc/macros.rs.html#37-47) macro is implemented in terms of it,
and several other parts of the [standard library](https://doc.rust-lang.org/src/alloc/sync.rs.html#314-318)
use it. 
[`Box::new`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.new) is the stable alternative to this;
however, in certain cases, it may not inline properly, which can cause the value to be instantiated on the
stack before being copied to the heap. This can blow up the stack.

However, box syntax has attracted criticism in its use of "placement new" syntax. It adds an unneeded level
of complexity to the language for something as specific as instantiating a `Box`. The purpose of this macro is
to "hide" this complexity from stable compiler users by integrating it into a macro, which is already a standard
part of the Rust language.

A benefit of this is that, if box syntax is significantly changed or even removed in the future, the `boxed`
macro could be updated to reflect this change to prevent ecosystem breakage. For instance, if we decided to
change box syntax to be similar to the `await` syntax (e.g. `x.box`), we could update the `boxed` macro to
expand to `x.box` instead of `box x`. Then, ecosystem users of `boxed` would not be affected by this change
at all.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `boxed` macro would be used in almost exactly the same cases that box syntax would be used in. Consider
the following case, where data must be directly allocated on the heap.

```rust
/// The Hectapus is like an octopus, but with 100 arms.
#[derive(Debug)]
struct Hectapus {
    arm_lengths: [u32; 100],
}

fn main() {
    // We can expect our hectapus tracking software to run on the embedded
    // computers hectapus scientists carry. Therefore, we need to initialize it
    // on the heap.
    let mut hectapus = box Hectapus { arm_lengths: [0; 100] };
    
    for i in 0u32..100 {
        hectapus.arm_lengths[i as usize] = i * 2;
    }

    println!("{:?}", &hectapus);
}
```

The following regex can be applied to replace instance of box syntax with the `boxed` macro:

```
s/box \(.*\)/boxed!(\1)/g
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `boxed` macro could be implemented in the standard library approximately as follows:

```rust
#[allow_internal_unstable(box_syntax)]
macro_rules! boxed {
    ($item: expr) => {{
        box $item
    }}
}
```

Of course, if box syntax is ever updated, this macro should be as well.

# Drawbacks
[drawbacks]: #drawbacks

Replacing instances of box syntax in the ecosystem with the `boxed` macro would likely be a source of
annoyance.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative to this would be the simply stabilize box syntax, as this mostly serves as a way of 
"pseudo-stablizing" the syntax so its technical benefits can be used in the broader ecosystem. However,
as this is likely years away, this macro serves as a way of bypassing box syntax.

# Prior art
[prior-art]: #prior-art

Replacing special syntax with a more generalized one has already been done before with the `raw_ref_macros`
feature in relation to `raw_ref_ops`. By replacing "magic" syntax with macro syntax, they were able to
stabilize their feature much earlier.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should there be a lint to replace `box x` with `boxed!(x)`?

# Future possibilities
[future-possibilities]: #future-possibilities

If a way to initialize memory on the stack without magic is introduced to the Rust language, `boxed` could be
modified to support that.
