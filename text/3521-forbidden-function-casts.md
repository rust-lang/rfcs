- Feature Name: `forbidden_function_casts`
- Start Date: (fill me in with today's date, 2023-11-12)
- RFC PR: [rust-lang/rfcs#3521](https://github.com/rust-lang/rfcs/pull/3521)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes to forbid all function-to-integer `as` casts in the next edition.


# Motivation
[motivation]: #motivation

Currently we allow casting of function pointers and function item types to any integer width, returning the function address. Casts to types that are smaller than `usize` truncate, while casts to bigger types zero-extend. Thus, a function-to-int cast is basically transitive — `f as u8` is the same as `f as usize as u8`.

This behavior can be surprising and error-prone. Moreover casting function item types is especially confusing, for example:
```rust
// This is an identity cast
// (`u8::MAX` is a `u8` constant)
let m = u8::MAX as u8;

// This returns the least significant byte
// of the function pointer address of `u8::max`
// (`u8::max` is a function returning maximum of two `u8`s)
let m = u8::max as u8
```

An example of this being a problem in practice can be seen in [rust-lang/rust#115511](https://github.com/rust-lang/rust/issues/115511):
```rust
// Code in `std::sys::windows::process::Command::spawn`
si.cb = mem::size_of::<c::STARTUPINFOW> as c::DWORD;
```

Instead of casting the *result* of `size_of` to `c::DWORD` the `size_of` function itself was cast, causing issues.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In edition 2024, function-to-integer casts are a hard error:
```rust
let ptr: fn() = todo!();
let addr = ptr as usize;   //~ ERROR
let addr_byte = ptr as u8; //~ ERROR
```

To get the old behavior you can cast fn pointer to a raw pointer first and then to an integer:
```rust
let addr = ptr as *const () as usize;
let addr_byte = ptr as *const () as usize as u8;
```

Those changes can be automatically applied by an edition migration lint.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

- On `edition >= 2024`
  - Casting function pointers or function definitions to an integer type is a hard error
      - Note that you can still cast function pointers and function item types to `*const V` and `*mut V` where `V: Sized`
      - Note that function item types can also be casted to function pointers
- On `edition < 2024`
  - Everything works as it used to
  - In cases where we produce an error on `edition >= 2024` an edition compatibility warning is emitted

# Drawbacks
[drawbacks]: #drawbacks

None known.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The simplest alternative is to do nothing, but it leaves a footgun that is easy to reach, which is probably not a good idea.

Another alternative is to only emit a lint, without making it an error on editions `>= 2024`. However, it seems like making `as` casts less powerful and less error prone is a desirable thing, and as such making it a hard error on newer editions is a good idea.

Another option would be to still allow casts of function pointers to `usize` (but disallow all other function->integer casts). This, however, still leaves possible bugs caused by accidentally casting a function to a `usize` instead of calling the function or using a similarly named constant.

We could also disallow more things:
- Disallow casting of function item types to raw pointers, requiring an intermediate cast to a function pointer. This may be desirable as it makes things even more explicit.
- Disallow casting on function pointers and function item types to any type using `as`. This removes all possible confusion, but requires us to provide a stable alternative (possibly a function accepting [`F: FnPtr`](https://doc.rust-lang.org/1.73.0/std/marker/trait.FnPtr.html), like [`FnPtr::addr`](https://doc.rust-lang.org/1.73.0/std/marker/trait.FnPtr.html#tymethod.addr)).

# Prior art
[prior-art]: #prior-art

- C supports casting function to differently sized integers
  - But both clang and gcc [produce a warning](https://godbolt.org/z/7bW3xc8Ec)
- C++ seems to disallow casts to differently sized integers
  - Both clang and gcc [produce an error](https://godbolt.org/z/Kd51aKaGh)
  - Casts to types that happen to have the same size are allowed

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- [ ] Should the lint on editions `< 2024` be allow-by-default or warn-by-default?

# Future possibilities
[future-possibilities]: #future-possibilities

### Disallowing transitive ptr-to-integer and integer-to-ptr casts

Similarly to function pointers, data pointers can also be cast to any integer width:

```rust
fn byte(ptr: *const ()) -> u8 {
    ptr as u8 // the same as `ptr as usize as u8` 
}

fn cursed(v: i8) -> *const () {
    v as *const () // sign-extends
                   // (same as `v as usize as *const ()`)
}
```

This can be confusing in a similar way to function-to-int casts and we could disallow it in a similar way too.
