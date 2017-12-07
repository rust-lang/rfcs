- Feature Name: const_fn_ptr
- Start Date: 2017-12-06
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Introduces the syntax `const fn(input_types..) -> return_type` as a type which 
`const fn`s may be coereced to.

# Motivation
[motivation]: #motivation

Currently, if you write:

```rust
#![feature(const_fn)]

const fn foo(f: fn(usize) -> usize) -> usize {
    f(1)
}

fn main() {}
```

The compiler greets you with:

```
error[E0015]: calls in constant functions are limited to constant functions, struct and enum constructors
 --> src/main.rs:4:5
  |
4 |     f(1)
  |     ^^^^

error: aborting due to previous error
```

With this RFC, the intent behind this is made possible with:

```rust
const fn foo(f: const fn(usize) -> usize) -> usize {
    f(1)
}
```

as well as:

```rust
type ConstFnPtr = const fn(usize) -> usize;

const fn foo(f: ConstFnPtr) -> usize {
    f(1)
}
```

This makes it possible to have higher-order-functions (HOFs) which are
also `const fn`s. Which makes `fn`s and `const fn`s more consistent and
increases symmetry.

This is a small addition to the language which enhances `const fn`s and allows
non-`const` `fn`s to restrict what a function taken as a parameter may do.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The term "const function pointer" or "const fn ptr" for short refers to the type
`const fn(input_types..) -> return_type`. Since it is a type, you may use it
in a type alias:

```rust
type MyFn = const fn(u32) -> u32;
```

This type alias may then be used in a `const fn` to take another `const fn`
as a parameter:

```rust
const fn twice(arg: u32, fun: MyFun) -> u32 {
    fun(fun(arg))
}

const fn add_one(arg: u32) -> u32 {
    arg + 1
}

fn main() {
    const THREE: u32 = twice(1, add_one);
    assert_eq!(THREE, 3);
}
```

You may also directly write:

```rust
const fn twice(arg: u32, fun: const fn(u32) -> u32) -> u32 {
    fun(fun(arg))
}
```

Const function pointers are not limited to use in `const fn`s.
You may also use them for normal `fn`s:

```rust
fn twice(arg: u32, fun: const fn(u32) -> u32) -> u32 {
    fun(fun(arg))
}
```

Since const function pointers are const, you may also write:

```rust
fn main() {
    const PTR: const fn(u32) -> u32 = twice;

    // ..
}
```

It is also important to note that the following is legal today:

```rust
fn main() {
    let fn_ptr : fn(u32) -> u32 = add_one;
}
```

That is: a `const fn` can coerce into a regular `fn` pointer.
And so with this RFC, the following would be legal:

```rust
fn main() {
    let hof_ptr_1 = const fn(u32, const fn(u32) -> u32) -> u32 = twice;
    let hof_ptr_2 = fn(u32, const fn(u32) -> u32) -> u32 = twice;
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Assuming we currently have (using [LBNF](https://github.com/BNFC/bnfc/blob/master/docs/lbnf.rst) notation):

```
separator Type ","

FnPtr. Type ::= "fn" "(" [Type] ")" "->" Type ;

-- And other production rules as well...
```

The following is also allowed (now):

```
ConstFnPtr. Type ::= "const" "fn" "(" [Type] ")" "->" Type ;
```

Semantically, only a `const fn fun_name(input_types) -> retr_type {..}` may be
used where a `const fn(input_types) -> retr_type` type is expected. Which
type checking (now) ensures. This is done as type checking of normal function
pointers are done, but also checking that the given `fn` is a `const fn`.

It should be noted that `const fn(.., const fn(x) -> y, ..) -> z` does not
coerce to `fn(.., fn(x) -> y, ..) -> z` as that would be unsound since
the outer `fn` may now be given a pointer to a normal `fn` which would execute
in a `const fn` context thereby subverting the rules of `const fn`.

# Drawbacks
[drawbacks]: #drawbacks

Some may argue that we don't need HOFs that are `const fn`.

# Rationale and alternatives
[alternatives]: #alternatives

This change is quite straightforward, the current function pointer syntax is
just reused for `const fn`s. Any other design would likely be inconsistent.

Not doing this would make HOFs that are `const fn` impossible.

# Unresolved questions
[unresolved]: #unresolved-questions

None. An exact implementation is the domain of stabilization.