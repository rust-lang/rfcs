- Feature Name: noalias
- Start Date: 2015-12-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a `noalias` language item and wrapper struct (similar to `non_zero`) to
mark raw pointers as `noalias`.

# Motivation
[motivation]: #motivation

Consider the following code:

```rust
#[no_mangle]
pub extern fn bx(mut x: Box<u8, Dummy>, mut y: Box<u8, Dummy>) -> u8 {
    *x = 11;
    *y = 22;
    *x
}

#[no_mangle]
pub extern fn rf(x: &mut u8, y: &mut u8) -> u8 {
    *x = 11;
    *y = 22;
    *x
}
```

where `Box` is an owning pointer and `Dummy` is an allocator that does not
perform any deallocation. (This setup was chosen so that the following assembly
is as simple as possible.) This produces the following output:

```
bx:
	movb	$11, (%rdi)
	movb	$22, (%rdx)
	movb	(%rdi), %al
	retq

rf:
	movb	$11, (%rdi)
	movb	$22, (%rsi)
	movb	$11, %al
	retq
```

In the `bx` case the value stored in `x` has to be reloaded because the pointer
wrapped by `y` might alias the pointer wrapped by `x`. That is, the second write
might overwrite the value stored in `x`.

# Detailed design
[design]: #detailed-design

Add a `noalias` language item and wrapper struct:

```rust
#[lang = "noalias"]
pub struct NoAlias<T: ?Sized>(*const T);
```

Memory access via a pointer `x`
[based on](http://llvm.org/docs/LangRef.html#pointeraliasing) a pointer `y`
wrapped in a `NoAlias` struct does
[not alias](http://llvm.org/docs/LangRef.html#noalias-and-alias-scope-metadata)
with memory access via pointers not based on `y`.

# Drawbacks
[drawbacks]: #drawbacks

None.

# Alternatives
[alternatives]: #alternatives

None.

# Unresolved questions
[unresolved]: #unresolved-questions

None at the moment.
