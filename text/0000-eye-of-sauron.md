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
- The "standard" in-place binary operators: `+=`, `-=`, `*=`, `/=`, `%=`, `^=`, `&=`, `|=`, `<<=`, `>>=`. 
- The "standard" unary operators: `!`, `-`. This does *NOT* include the dereference operator `*`.
- The indexing operator `...[...]`

After this RFC, operator type-checking behaves as follows:

Both the LHS and the RHS (if binary) of the operator are first type-checked with no expected type.

This differs from rustc 1.20, in which an expected type was sometimes propagated from the LHS into the RHS, potentially triggering coercions within the RHS. I should probably come up with an example in which this matters.

Then, it performs method-style lookup with the following parameters:

1. The following dispatchable arguments: argument #0 has uncoerced type `lhs_ty`, and, if this is a binary operator, argument #1 has uncoerced type `rhs_ty`.
2. For just the LHS of an indexing operator (the `X` in `X[Y]`), and both operands of a comparison operator (i.e. `==`, `<`, `<=`, `!=`, `>=`, `>`), adjustment lists must match the following regular expression:
    ```
    "Deref"* "Autoref(Immutable)" "ConvertArrayToSlice"?
    ```
    For just the LHS of in-place binary operators, adjustment lists must match the following regular expression:
    ```
    "Deref"* "Autoref(Mutable)" "ConvertArrayToSlice"?
    ```    
    For all other operators (including the RHS of an indexing operator), adjustment lists must match the following regular expression:
    ```
    "Deref"* ( "Autoref(Immutable)" "ConvertArrayToSlice"? )?
    ```
3. One method candidate - this is the obvious operator method. For indexing, this is always `Index::index` - if needed, it will be "upgraded" to `IndexMut::index_mut` through a mutability fixup (this might matter for some edge cases in inference, but rustc 1.20 is inconsistent in that regard - sometimes it can combine the lookup and the mutability fixup).

Then, if indexing was used in a mutable context, the [Mutablity Fixup](#mutability-fixup) will be applied to it.
    
## Method-Style Lookup

This description depended on a few details of method type-checking, some of them slightly modified. I normally would have documented the few needed *changes* to method lookup, but it is ill-documented today, so here's a description of it:

Method lookup is parameterized on several things:
1. An ordered list of (argument #, type) list of unadjusted dispatchable arguments (before this RFC, there could only be 1 dispatchable argument - the method receiver - but the logic generalizes).
2. For each dispatchable argument, the set of usable adjustment lists for it.
3. The set of method candidates - this is a set of methods, one of them is to be selected.

Method-style lookup proceeds as follows:

### Step 1 - Adjustment list set determination

First, final set of `(adjustment list, adjusted argument type)` pairs is determined for each dispatchable argument

For each usable adjustment list for that argument:
- If it can be successfully applied to the (unadjusted) argument type, add the adjustment list along with the adjusted argument type to the final set.
- If it can be proven to fail when applied to the type, ignore it.
- Otherwise, this is an ambiguity and a compilation error.

#### EXAMPLE 1.

For example, this code yields a compilation error (in all versions of rustc):
```Rust
    trait Id { fn id(self); }
    impl<T> Id for T { fn id(self) {} }

    let mut x = None;
    
    if let Some(ref x) = x {
        // x: &_ here
        // the adjustment list `Deref Deref` works only if `_: Deref`, and that
        // can either fail or succeed, so we get an error. Note that the *empty*
        // adjustment list would have worked just fine, but we determine the
        // set of adjustment lists first
        x.id(); //~ ERROR
    }
    
    x = Some(());
```

Similar examples could be created for operator dispatch (that would not fail in rustc 1.20), and I hope these will not be much of a problem in practice. 

### Step 2 - Adjustment list selection

Then, the best assignment of adjustment lists is picked from the cartesian product of the adjustment list sets - one for each dispatchable argument.

The picked assignment is the first assignment (in lexicographic order) from the cartesian product such that is at least 1 candidate in the candidate set that might apply to that assignment.

If no such assignment exists, it is a compilation error.

A candidate might apply to an assignment unless subtyping the candidate's dispatchable argument types with the assignment's respective adjusted dispatchable argument types proves that one of the candidate's predicates can't hold (if the subtyping can't be done, that vacuously proves that the predicates can't hold).

#### EXAMPLE 2. (Arithmetic)

For example, with operators:
```Rust
trait Add<T> {
    type Output;
    fn add(self, rhs: Self) -> Self::Output
        // there's a `where Self: Add<T>` implicit predicate
        ;
}
/* A */ impl Add<u32> for u32 { type Output=u32; /* .. */ } 
/* B */ impl<'a> Add<&'a u32> for u32 { type Output=u32; /* .. */ }
/* C */ impl<'b> Add<u32> for &'b u32 { type Output=u32; /* .. */ }
/* D */ impl<'a, 'b> Add<&'a u32> for &'b u32 { type Output=u32; /* .. */ }
/* E */ impl Add<i32> for i32 { type Output=i32; /* .. */ }
/* F */ impl<'a> Add<&'a i32> for i32 { type Output=i32; /* .. */ }
/* G */ impl<'b> Add<i32> for &'b i32 { type Output=i32; /* .. */ }
/* H */ impl<'a, 'b> Add<&'a i32> for &'b i32 { type Output=i32; /* .. */ }
// <possible other impls>

println!("{}", 1 + 1);
```

Our candidate method is `Add::add`, and both arguments have (different) inference variable types `$int0` and `$int1`.

The usable adjustment lists are of the form `"Deref"* ( "Autoref(Immutable)" "ConvertArrayToSlice"? )?`. Because `Deref` and `ConvertArrayToSlice` can't be used on integers, we are left with the following adjustment lists (they set is identical for both locals except the variable changes names):

```
([], $int0/$int1)
([Autoref(Immutable)], &$int0/&$int1)
```

The cartesian product, in order, is
```
(arg0=([], ty=$int0), arg1=([], ty=$int1))
(arg0=([Autoref(Immutable)], ty=&$int0), arg1=([], ty=$int1))
(arg0=([], ty=$int0), arg1=([Autoref(Immutable)], ty=&$int1))
(arg0=([Autoref(Immutable)], ty=&$int0), arg1=([Autoref(Immutable)], ty=&$int1))
```

For the first assignment, we see that our candidate might apply: `$int0: Add<$int1>` can hold using both impls `A` and `E`, and there are no other interesting predicates, so we select the first adjustment list and candidate.

Later on, arithmetic fixup gives us a return type, and at the end inference fallback picks `i32` for the variable.

#### EXAMPLE 3. (Reference arithmetic)

Suppose we are now checking the `>`-operator in following method:
```Rust
trait PartialOrd<Rhs> {
    fn lt(&self, other: &Rhs) -> bool;
    // (irrelevant code omitted)
}

/* I */ impl PartialOrd<i32> for i32 { /* .. */ }
/* J */ impl<'a, 'b, A, B> PartialOrd<&'b B> for &'a A where A: PartialOrd<B>
    { /* .. */ }

fn foo(v: Vec<i32>) {
    v.iter().filter(|x: &&i32| x > 0);
}
```

In older versions of rustc, this would fail and require playing with inference to make it work. With the new operator semantics, let's see how it works.

`>` is a by-ref operator, so our adjustment lists must include an autoref. For the LHS, we can have either 0, 1, or 2 derefs, and `ConvertArrayToSlice` is irrelevant, so we have the following CLS:
```
([Autoref(Immutable)], &&&i32)
([Deref, Autoref(Immutable)], &&i32)
([Deref, Deref, Autoref(Immutable)], &i32)
```

For the RHS, we can't have any non-zero number of derefs, s 
```
([Autoref(Immutable)], &$int0)
```

We then go over the cartesian product:
```
lhs=([Autoref(Immutable)], &&&i32),
rhs=([Autoref(Immutable)], &$int0)
    - subtype `&Self <: &&&i32, &RHS <: &$int0`
    - we have Self=&&i32, RHS=$int0
    - &&i32: PartialOrd<$int0> can't hold, ignoring
lhs=([Deref, Autoref(Immutable)], &&i32)
rhs=([Autoref(Immutable)], &$int0)
    - subtype `&Self <: &&i32, &RHS <: &$int0`
    - we have Self=&i32, RHS=$int0
    - &i32: PartialOrd<$int0> can't hold, ignoring
lhs=([Deref, Deref, Autoref(Immutable)], &i32)
rhs=([Autoref(Immutable)], &$int0)
    - subtype `&Self <: &i32, &RHS <: &$int0`
    - we have Self=i32, RHS=$int0
    - i32: PartialOrd<$int0> can hold (impl I), success!
```

So we perform 2 derefs of the LHS and 0 derefs of the RHS (plus 2 autorefs) and succeed.

#### EXAMPLE 4. (Adding strings)

One nice thing this RFC solves is adding strings.

The current (and future) relevant impls are:

```Rust
/* K */ impl Deref for String { type Target = str; /* .. */ }
/* L */ impl<'a, B: ?Sized> Deref for Cow<'a, B> { type Target = B; /* .. */ }
/* M */ impl<'a> Add<&'a str> for String { type Output = String; /* .. */ }
/* N */ impl<'a> Add<Cow<'a, str>> for Cow<'a, str> { type Output = Self; /* .. */ }
/* O */ impl<'a> Add<&'a str> for Cow<'a, str> { type Output = Self; /* .. */ }
```

Now, `String + &str` and `Cow + &str` obviously work. I want to look at the `String + String` and `Cow + Cow` cases.

For `String + String`, we have the following CLS for both the LHS and RHS:
```
([], String)
([Autoref(Immutable)], &String)
([Deref], str) - yes this is unsized
([Deref, Autoref(Immutable)], &str)
```

We then go over the cartesian product:
```
lhs=String, rhs=String - no match
lhs=String, rhs=&String - no match
lhs=String, rhs=str - no match
lhs=String, rhs=&str - match!
```

So we do a deref + autoref of the RHS. This means that only the LHS will be moved out - the RHS will only be borrowed, so you can write:
```Rust
let x = "foo".to_string();
let y = "bar".to_string();
let z = x + y;
println!("{} {}", z, y);
```

This works just as well for `Cow + String`. When adding `Cow + Cow`, the situation is different:
```
lhs=Cow<'a, str>, rhs=Cow<'a, str> - match! (using impl N)
```

This means that we are doing by-value addition, and will move out the RHS (same as today). Removing impl N would be a breaking change at this moment, but it would improve UX so it might we worth investigating.

#### EXAMPLE 5. (Adding field elements, refs only)

This is the case from Isis Lovecruft's "eye of sauron" example. The relevant impls are just:

```Rust
struct FieldElement;
/* P */ impl<'a, 'b> Add<&'b FieldElement> for &'a FieldElement {
    type Output = FieldElement;
    // ..
}
```

And we are adding 2 field elements `a + b`. There are no derefs, so the CLS for both the LHS and RHS are:
```
([], FieldElement)
([Autoref(Immutable)], &FieldElement)
```

And we go over the cartesian product, and pick the impl with both autorefs:
```
lhs=FieldElement, rhs=FieldElement - no match
lhs=FieldElement, rhs=&FieldElement - no match
lhs=&FieldElement, rhs=FieldElement - no match
lhs=&FieldElement, rhs=&FieldElement - match!
```

We aren't doing any moves, and everything works!

### Step 3 - Candidate selection

After adjustments are selected, the candidate is selected (for operators, this is trivial, because there is only ever 1 candidate) according to the following rules:

- If there is exactly 1 candidate, it is selected
- If there are multiple candidates, but exactly 1 high-priority candidate, it is selected.
- Otherwise, this is a compilation error.

### Step 4 - Fixups

Then following fixups are made. They do not affect adjustment or candidate selection.

#### Mutability Fixup

If we performed a mutable autoref, or are performing overloaded indexing in a mutable context, we need to apply a *mutability fixup* to the adjustments and to other lvalue components on the way.

This proceeds by replacing immutable borrows in the lvalue path with mutable borrows, and adding `DerefMut` and `IndexMut` trait bounds when appropriate. If the trait bounds fail, this is of course a compilation error.

#### Arithmetic Fixup

If an arithmetic operator was used, and both types are integer or float  inference variables, their types are unified as if there existed an impl generic over integer inference variables, e.g.
    ```Rust
    impl<I: IntegerType> Add<I> for I { // or modify for other operators
        type Output = I;
        // ..
    }
    ```
    
This is required in order to make `1 + 2` (both parameters are integer inference variables) be known to be an integer before integer defaulting.

## Adjustments

For the purpose of method lookup, adjustments are as follows: for the purpose of overloaded operators, an adjustment is defined as follows:

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
~ is eqty, !~ is "not eqty", "!:" is "no impl for any substitution of inference variables".

Adjust rules:

T : Deref
------------
adjust(Deref, T) = Success(<T as Deref>::Target)

T !: Deref
------------
adjust(Deref, T) = Failure

T type
------------
adjust(Autoref(Immutable), T) = Success(&T)
adjust(Autoref(Mutable), T) = Success(&mut T)

T, E types
n constant usize
T ~ &[E; n]
------------
adjust(ConvertArrayToSlice, T) = Success(&[E])

T, E types
n constant usize
T ~ &mut [E; n]
------------
adjust(ConvertArrayToSlice, T) = Success(&mut [E])

T type
∀E type, n constant usize. T !~ &[E; n]
∀E type, n constant usize. T !~ &mut [E; n]
------------
adjust(ConvertArrayToSlice, T) = Failure

And `adjust_list` is just adjust mapped over lists:
------------
adjust_list([], T) = Success(T)

adjust(a, T) = Failure
------------
adjust_list([a, as], T) = Failure

RESULT result
adjust(a, T) = Success(U)
adjust_list([as], U) = RESULT
------------
adjust_list([a, as]) T = RESULT
```

The intent of the "included middle"-style rules is that if we can't determine whether we can apply an adjustment due to inference variables, we can't determine success or failure (and that should result in a compilation error).

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

### Appendix A. Method Dispatch

This is supposed to describe method dispatch as it was before this RFC. 

TBD

