- Feature Name: `arrow_deref`
- Start Date: 2024-02-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC improves ergonomics for pointers in unsafe Rust. It adds the RArrow token as a single-dereference member access operator. `x->field` desugars to `(*x).field`, and `x->method()` desugars to `(*x).method()`.

# Motivation
[motivation]: #motivation

In unsafe Rust, there has long been a lack of ergonomics. With advancements such as [raw_ref_op](https://github.com/rust-lang/rfcs/pull/2582) we are coming closer to a more ergonomic unsafe Rust.

Auto-deref does not operate on raw pointers. This is because we want a clear boundary between unsafe and safe. We want dereferencing of raw pointers to be explicit. There is a reason that auto-deref exists for references - ergonomics.

It is possible to have both a clear boundary and ergonomics for raw pointers. The problem stems from the affix kind of the asterisk operator - prefix. Because the dot operator has higher precedence, we are left with excess parentheses. Example:

```rust
(*(*(*pointer.add(5)).some_field).method_returning_pointer()).other_method()
```

What we need here is either a suffix operator, or an infix operator. With the already existing RArrow token, we could have 

```rust
pointer.add(5)->some_field->method_returning_pointer()->other_method()
```

This is identical to C and C++, which is also great for parity.

One common non-solution to this problem is "encapsulate the unsafe code". First off, it does not address the ergonomics. Second, it is not possible in domains with irreducible encapsulations. Most notable prevalent domains with irreducible encapsulations are:
1. Non-trivial intrusive data structures.
2. Interoperability with complex systems written in C.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The arrow operator in Rust is usually used to mark the return type of functions, function pointers and closures. With this RFC, it can also be used to dereference a raw pointer to an object with a field or method. For instance, if we have the type

```rust
struct S {
    next: *mut S,
    value: i32,
}
impl S {
    fn increment(&mut self) {
        self.value += 1;
    }
}
```

and a pointer variable `p` of type `*mut S`, we can write

```rust
p->next->increment();
```

and

```rust
p->next->value *= 2;
```

instead of

```rust
(*(*p).next).increment()
```

and

```rust
(*(*p).next).value *= 2;
```

respectively. For an expression `X` and field `F`, `X->F` is exactly equivalent to `(*X).F`. For method `M`, `X->M()` is exactly equivalent to `(*X).M()`. It dereferences X precisely once, unlike auto-deref that can dereference multiple, or zero times.

Especially for long and nested expressions `X`, working with the arrow is more ergonomic. It makes unsafe code easier to read, understand, and maintain. 

If you are coming from a C or C++ background, the arrow operator for pointers in unsafe Rust behaves identically to the arrow operator for pointers in C and C++.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The author of this RFC has implemented the feature with ~50 lines of code in `expr.rs`, `errors.rs` and `messages.ftl` in `rustc_parse`. It involves two steps:
1. Accepting the `RArrow` token in tandem with the `Dot` token, in `parse_expr_dot_or_call_with_`. 
2. Renaming `parse_dot_suffix` to `parse_dot_or_arrow_suffix` and passing a unary `UnOp::Deref` expression in `self_arg`.

There are no grammar ambiguities with respect to other instances of `RArrow`, such as `Fn() -> T`.

# Drawbacks
[drawbacks]: #drawbacks

The RArrow token could have other future use cases.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The proposal makes unsafe code, that which is the most safety-critical code, easier to read, understand, and maintain.

It also prevents preferring references over raw pointers. This prevents common mistakes that create UB by simultaneous mutable references.

As discussed in the article [Rust's Unsafe Pointer Types Need An Overhaul](https://faultlore.com/blah/fix-rust-pointers/), the Tilde token could be used for walking field pointers of different types without changing the level of indirection. The proposed arrow operator is different.
The arrow dereferences and yields a place expression. This is important because it is the only way to completely eliminate excess parentheses. Suppose we used the Tilde token for obtaining pointers to fields. Then the example before would be written as:

```rust
(*(**pointer.add(5)~some_field).method_returning_pointer()).other_method()
```

Which does not solve the problem.

# Prior art
[prior-art]: #prior-art

This exists in both C and C++. If for example C had neither the arrow nor an alternative auto-deref, writing C code would be quite cumbersome. Early development of Zig also incorporated the utility of non-prefix operators for dereference and auto-deref. Like 
[Rust itself](https://github.com/rust-lang/rfcs/pull/102). However, Rust makes a clear distinction between unsafe and safe, where we need to be cautious.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

One unresolved question is whether the arrow operator should only be capable of dereferencing raw pointers. Should it be a sugar that desugars to `(*x).y`, no matter the type of `x`, or should `x` only be allowed to be a raw pointer? Implementing this requires an extra type-checking step in addition to the `rustc_parse` implementation outlined above.

Maybe a trait called UnsafeDeref? This way it can be implemented for both raw pointers and NonNull<T>.

# Future possibilities
[future-possibilities]: #future-possibilities

There have been many discussions over the years.

[Need for -> operator for Unsafe Code Guidelines](https://internals.rust-lang.org/t/need-for-operator-for-unsafe-code-guidelines/10022)

[Add arrow operator as sugar for (*var)](https://github.com/gpuweb/gpuweb/issues/4114)

[Rust's Unsafe Pointer Types Need An Overhaul](https://faultlore.com/blah/fix-rust-pointers/)
