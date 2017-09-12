- Feature Name: eye_of_sauron
- Start Date: 2017-09-11
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow for method-dispatch-style trait-driven autoderef and autoref in operators.

# Motivation
[motivation]: #motivation

In today's Rust, working with operators is annoying, because they are supposed to be a "lightweight" syntax like methods and field accesses, but unlike them, they do not have coercions on their LHS and therefore require explicit autoderefs and autorefs.

One common example where this is a nuisance is when using iterator adapters, working which can easily create multiple-reference types such as `&&u32`. For example:
    
```Rust
    let v : Vec<u32> = vec![0, 1, 2, 3];
    // Why can't I just write `x > 1`? Why do I have to ** for the compiler???
    v.iter().filter(|x| **x > 1).count();
```

There are several cases where these operators are used. The most popular case is indexing associative maps:

```Rust
    use std::collections::HashMap;
    let x: HashMap<_, _> =
        vec![(format!("hello"), format!("world"))].into_iter().collect();
    let s = format!("hello");
    
    // I would like to write...
    println!("{}", x[s]);
    // But instead, I have to write...
    println!("{}", x[&s]);
```

In this case, these are merely annoying papercuts. In some other cases, they can be a much worse problem.

One of thim is Isis Lovecruft's "Eye of Sauron" case, which is a problem when using non-`Copy` bignums:

```Rust,ignore
struct Bignum(..);
// fair enough
impl<'a> Add for &'a Bignum {
    type Output = Bignum;
    // ...
}
// ...

// I can't see what my code is doing! It looks like one big piece of &.
let a = &(-(&A)) * &(&one + &nrr).invert();
let z = &u * &(&(&u.square() + &(&A * &u)) + &one);

// Would be better:
let a = (-A) * (one + nrr).invert();
let z = u * (u.square() + A * u + one);
```

Allowing method-style autoref could make operator uses as clean as methods and fields, making code that uses them intensively far more ergonomic. It's also a fairly non-invasive extension.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Like methods, operators and indexing support automatic referencing and dereferencing. When you use an operator, the compiler will automatically add `&` and `*` operators to match the signature of the operator.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

"Operators" here refers to several typeck operators:
- The "standard" eager binary operators: `+`, `-`, `*`, `/`, `%`, `^`, `&`, `|`, `<<`, `>>`, `==`, `<`, `<=`, `!=`, `>=`, `>`.
- The "standard" unary operators: `!`, `-`. This does *NOT* include the dereference operator `*`.
- The indexing operator `...[...]`

Operator type-checking behaves similarly to method type-checking. It works as follows:

## Step 1 - Subexpression checking

Both the LHS and the RHS (if binary) of the operator are first type-checked with no expected type.

This differs from rustc 1.20, in which an expected type was sometimes propagated from the LHS into the RHS, potentially triggering coercions within the RHS. I should probably come up with an example in which this matters.

## Step 2 - Adjustment selection

Afterwards, an adjustment list is selected for both operands as follows:

Adjustment lists for the LHS of an indexing operator (the `X` in `X[Y]`) are selected from these matching the following regular expression:
```
"Deref"* "Autoref(Immutable)" "ConvertArrayToSlice"?
```

Adjustment lists for all other operands (including the RHS of indexing operator, as well as all operands of all other operators) are selected from these matching the following regular expression
```
"Deref"* ( "Autoref(Immutable)" "ConvertArrayToSlice"? )?
```

The adjustment lists selected are the lexicographically first pair of adjustment lists `(lhs_adjust, rhs_adjust)` (or with an unary op, just the `lhs_adjust`) such that
A1. Both adjustment lists match the relevant regular expressions
A2. Both adjustment lists must be valid to apply to their operand types. If, due to the presence of inference variables, it can't be determined whether these adjustment lists would be valid to apply, and we didn't find a smaller adjustment list that would apply, that is a compilation error (this is the "can't autoderef because of inference variables" case). 
A3. After applying both adjustment lists, the adjusted operand types are a potential match for the operator trait (if there is an ambiguity because of inference variables, it is counted as a match).
   A3.1. NOTE: the operator trait for overloaded indexing is `Index`, not `IndexMut`, even if indexing is done in a mutable context. rustc 1.20 is inconsistent in that regard.
   
## Step 3 - Fixups

After adjustments are selected, the following fixups are made. They do not affect adjustment selection.

### Mutability Fixup

If overloaded indexing is used in a mutable context, the `Autoref(Immutable)` adjustment of the LHS is replaced with an `Autoref(Mutable)` adjustment, and the entire chain is required to be consistent with the new mutability (using the `DerefMut` and `IndexMut` traits when needed). If they can't be, this is a compilation error.

### Arithmetic Fixup

If an arithmetic operator was used, and both types are integer or float  inference variables, their types are unified as if there existed an impl generic over integer inference variables, e.g.
    ```Rust
    impl<I: IntegerType> Add<I> for I { // or modify for other operators
        type Output = I;
        // ..
    }
    ```
    
This is required in order to make `1 + 2` (both parameters are integer inference variables) be known to be an integer before integer defaulting.

## Operator Adjustments

These are basically the same as method adjustments, but because these are underdocumented: for the purpose of overloaded operators, an adjustment is defined as follows:

```Rust
type Adjustments = Vec<Adjustment>;
#[derive(PartialOrd, Ord, PartialEq, Eq)]
enum Mutability {
    Immutable,
    Mutable
}

#[derive(PartialOrd, Ord, PartialEq, Eq)]
enum Adjustment {
    Autoref(Mutability),
    ConvertArrayToSlice,
    // this must be last, and means that k+1 derefs is always > k derefs
    Deref,
}
```

Adjustments have the following effect on types
```
adjust(Deref, ty) = /* do immutable autoderef */
adjust(Autoref(Immutable), ty) = Some(`&$ty`)
adjust(ConvertArrayToSlice, &[ty; N]) = Some(`&[$ty]`)
adjust(ConvertArrayToSlice, &mut [ty; N]) = Some(`&mut [$ty]`)
adjust(ConvertArrayToSlice, _) = None

adjust_list(adjustments, ty) =
    let mut ty = ty;
    for adjustment in adjustments {
        ty = if let Some(ty) = adjust(adjustment, ty) {
            ty
        } else {
            return None;
        }
    }
    Some(ty)
```

And have the obvious effect on values. Adjustments are ordered using the standard lexicographical order.

# Drawbacks
[drawbacks]: #drawbacks

### Inference Complexity

The 2-argument inference adds more complexity to type-checking. It probably has some "interesting" and unexpected uses in some situations, and we need to gain some experience with it before stabilization.

### Unexpected References

The automatic references can make it less obvious when values are passed by reference or by value. Because operator traits are very general, the created autorefs can easily escape outside of the operator.

First, I don't feel this is worse than the situation with method calls - they can also create unexpected references under pretty much the same situations.

Second, and similarly to method calls, this is somewhat alleviated by the compiler picking the by-value implementation if the type matches, possibly causing move errors even if autoref would have worked:

```Rust,ignore
struct Bignum(..);
// fair enough
impl<'a> Add for &'a Bignum {
    type Output = Bignum;
    // ... impl, allocating
}
impl<'a> Add<&'a Bignum> for Bignum {
    type Output = Bignum;
    // ... impl, not allocating
}

let a = bignum1 + bignum2; // moves bignum1
let b = bignum1 + 1; //~ ERROR use of moved value

let c = &bignum3 + bignum4; // allocates, does not use bignum3.
let d = bignum3 + 1; // works
```

However, this actually might tempt people to write only by-ref allocating impls for their types to avoid "unexpected" moves. We might want to gather more experience here.

# Rationale and Alternatives
[alternatives]: #alternatives

The "Eye of Sauron" case is very annoying for mathematical code, and therefore is something we want to fix. This fix feels rather elegant and is contained to binary operators.

## Alternative Solutions

We need to use special coercion logic rather than the "standard" expected type propagation because we are coercing on the basis of trait impls. Therefore, options such as [RFC 2111] will not help in all cases. We could instead try to add trait-impl-based propagation, but that would not solve the "binary" nature of operator traits.

The essential reason we need to consider the "binary" nature is because of the `v.iter().filter(|x| x > 1)` case. If we did method-style inference, we would pick 0 dereferences for `x` - using the `PartialOrd<&&u32> for &&u32` impl combination - and will then fail when trying to coerce the `1`.

If we already want a "binary" solution, this "double method lookup" is the simplest solution I know of.

# Unresolved questions
[unresolved]: #unresolved-questions

## Flexibility Points

These are small places where the RFC can be changed. I wrote down the version I liked the most, and this RFC was getting too long and pedantic even without me trying to include alternatives.

Possible alternatives are:

### Different set of operators

Indexing is included in this feature basically because it behaves like the other operators. It's possible that this should not be done.

### Different way to pick the adjustment list

Lexicographic ordering feels like the right way to pick the adjustment list, but it might have unexpected edge cases which might justify a more complicated ordering.

### Extended arithmetic fixup

Because the arithmetic fixup does not apply with references, there could be inference issues with integer variables:
```
let x = &1 + &2; // this uses the <&i32 as Add<&i32>> impl,
                 // so `x` has type `_` before integer fallback
x.foo(); //~ ERROR cannot infer the type
```
therefore, We might want to extend the arithmetic fixup to references to integer variables in some way.

### Mutable autorefs

We might want to allow mutable autorefs in more cases. That would be more consistent with method lookup, but is probably a bad idea because it allows for too easy implicit mutability.

### Lifetime limits

[RFC 2111] limits the lifetime of implicit autorefs to the containing expression. We might also want to do that here to avoid confusing escaping autorefs.

### Improving impls

Instead of adding autoderef for operators, we could try adding enough trait impls for operators.

For example, with lattice specialization, we could have a trait with the following impls:
```Rust
trait MyAutoderef<T> {}
impl<T> MyAutoderef for T {}
impl<'a, U, V> MyAutoderef<U> for V where
    V: Deref,
    V::Target: MyAutoderef<U> {}
// this is the special "lattice join" impl that is needed
// to make the compiler shut up.
impl<'a, T> MyAutoderef<T> for T
    where T: Deref,
          T::Target: MyAutoderef<T>
{}
```

And then we could have impls
```Rust
impl<T: MyAutoderef<u32>, V: MyAutoderef<u32>> Add<U> for V {
   // ...
}
```

However:
A. Lattice specialization requires that specialization will work well, which might take quite a bit of time to figure out.
B. As written, the operator impls will all conflict with each-other and all other impls of operators and wildly break type inference. For every pair of types `U` and `V`, you'll have to convince the compiler that a type couldn't be both `MyAutoderef<U>` and `MyAutoderef<V>`, which would be non-trivial, or find some way to prioritize the impls, which will add more complexity and won't work cross-crate.
C. Without autoref, types that are not `Copy` will be moved. For example, with this RFC you can index a `HashMap` with a `String` *without moving the `String`*, which can't be done by an impl.

### General Coercions

We might want to allow for more general coercions than autoref and autoderef. For example, function item to function pointer coercions. Is there any use for that? Does it bring disadvantages?

[RFC 2111]: https://github.com/rust-lang/rfcs/pull/2111
