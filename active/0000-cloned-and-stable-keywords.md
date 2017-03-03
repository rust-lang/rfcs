- Start Date: 2014-06-16
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Add the keywords `cloned` and `stable` to enable an implicit move-optimization similar to what is made possible in C++ by the combination of rvalue references and function overloading.

# Motivation

We don't want to cause unnecessary memory allocations like these:
```
#[deriving(Clone)]
struct Vector {
    coordinates: Vec<int>
}

impl Mul<int, Vector> for Vector {
    fn mul(&self, rhs: &int) -> Vector {
        let mut new_coordinates = self.coordinates.clone();
        for c in new_coordinates.mut_iter() {
            *c *= *rhs;
        }
        Vector { coordinates: new_coordinates }
    }
}

fn get_vector() -> Vector {
    let v = Vector { coordinates: vec!(1, 2, 3) };
    v * 2 * 5
}
```
The last line in the body of `get_vector` causes two memory allocations, when that last line shouldn't allocate at all. It should only move `v` around and modify the coordinates in place.

# Detailed design

The interplay of two new keyword, `cloned` and `stable`, make the C++ like implicit move-optimizations possible. What's more, these keywords also make the trait-system more expressive by letting programmers declare an intent incommunicable before, namely, the intent that you don't care how a certain argument to a function declared by a trait is passed in the signature of the implementor function as long as the act calling that function with a certain variable as that argument doesn't logically change the state of that passed variable. Then, the ability to declare this intent is made immensely more useful by the `cloned` keyword.

## cloned
A function argument (i.e. a runtime parameter to a function) can be labeled `cloned` if the type of the argument is not a reference nor a mutable reference and the type of the argument implements the `Clone` trait. The keyword `cloned` must appear before the argument name separated from it by whitespace, or, in-place of the argument name if the argument name is omitted. The effect of labeling an argument `cloned` is that calling such a function with some variable `x` passed to it as this parameter labeled `cloned` behaves as if the expression `x.clone()` were passed to it instead of `x`. But, there are many situations where the effect of passing just `x` has the same computational behavior as passing `x.clone()`, and in those situations the language guarantees that the unnecessary implicit clone is omitted.

Here are examples of valid function signatures using `cloned` argument(s):
```
fn foo<T>(cloned a: Box<T>) {}

fn bar<T: Clone>(cloned a: T) {}

fn baz(cloned: Vec<int>) {}

#[deriving(Clone)]
struct S;

impl S {
    fn method(cloned self) {}
}
```

Whereas these are examples of functions that would cause a compile-time error:
```
fn foo<T>(cloned a: &Box<T>) {} // `cloned` can't be passed as a reference

fn bar<T>(cloned a: T) {} // `T` doesn't implement the `Clone` trait

fn baz(cloned cloned: Vec<int>) // keyword `cloned` used as an identifier
```

The next example illustrates the situations where the implicit clone-method call is omitted when a variable is passed as an argument labeled as `cloned`. The code in Example-1 would effectively be lowered by the compiler to the code in Example-2:

**Example-1:**
```
fn foo_cloning(cloned a: Box<int>) {}

fn user_code(a: Box<int>) {
    foo_cloning(a); // `a` cloned due to non-last use
    foo_cloning(a); // `a` not cloned due to last use before assignment
    a = box 123;
    foo_cloning(a);     // `a` cloned due to non-last use
    a = foo_cloning(a); // `a` not cloned due to last use before assignment
    foo_cloning(a);     // `a` cloned due to non-last use
    foo_cloning(a);     // `a` not cloned due to last use
}
```

**Example-2:**
```
fn foo_moving(a: Box<int>) {}

fn compiler_code(a: Box<int>) {
    foo_moving(a.clone()); // `a` cloned due to non-last use
    foo_moving(a);         // `a` not cloned due to last use before assignment
    a = box 123;
    foo_moving(a.clone()); // `a` cloned due to non-last use
    a = foo_moving(a);     // `a` not cloned due to last use before assignment
    foo_moving(a.clone()); // `a` cloned due to non-last use
    foo_moving(a);         // `a` not cloned due to last use
}
```

## stable
A function argument (i.e. a runtime parameter to a function) in a function signature that is a part of a definition of a trait can be labeled `stable` if the type of the argument is not a reference nor a mutable reference. The keyword `stable` must appear before the argument name separated from it by whitespace, or, in-place of the argument name if the argument name is omitted. If at least one of the trait-function arguments is labeled `stable`, then that function is not allowed to have a default implementation. If a trait `Foo` declares a function `foo` which takes an argument that is labeled `stable`, is named `arg` and is specified to be of a concrete type `A`, then a type which implements `Foo` is allowed to implement `foo` in a few different function signatures. The implementation of `foo` may specify the `arg` argument as one of the following:

1) `arg: &A`  
2) `arg: A` (but only if `A` implements the `Copy` trait)  
3) `cloned arg: A` (but only if `A` implements the `Clone` trait)

Here are some examples of valid traits using `stable`:
```
trait Foo {
    fn foo<T: Copy>(stable arg: T);
}

trait Bar {
    fn bar<T: Clone>(stable arg: T);
}

trait Baz {
    fn baz<T>(stable arg: T);
}

```

And here are all the possible valid function signatures that could be used when a type implements one of the traits above:
```
// in implementing Foo
fn foo<T: Copy>(arg: T)
fn foo<T: Copy>(arg: &T)
fn foo<T: Copy>(cloned arg: T)
```

```
// in implementing Bar
fn bar<T: Clone>(arg: &T)
fn bar<T: Clone>(cloned arg: T)
```

```
// in implementing Baz
fn baz<T>(arg: &T)
```

A variable passed to a function as an argument labeled `stable` is eligible to automatic referencing/dereferencing just the same as the `self` arguments are. The reason for this is that generic functions such as the following must work:
```
trait Qux {
    fn qux(&self, stable arg: int);
}

fn possibly_auto_referencing<T: Qux>(q: &T, value: int) {
    q.qux(value); // `value` may need to be auto-referenced depending on `T`
}

fn possibly_auto_dereferencing<T: Qux>(q: &T, value: &int) {
    q.qux(value); // `value` may need to be auto-dereferenced depending on `T`
}
```

Now, with these two new keywords, the C++ like implicit move-optimization could be accomplished in our earlier motivating example with the following two changes:

1) Change the definition of the `Mul` trait to this:
```
pub trait Mul<RHS, Result> {
    fn mul(stable self, stable rhs: RHS) -> Result;
}
```

2) Change our implementation of `Mul` for `Vector` to this:
```
impl Mul<int, Vector> for Vector {
    fn mul(cloned self, rhs: int) -> Vector {
        for c in self.coordinates.mut_iter() {
            *c *= rhs;
        }
        self
    }
}
```

# Drawbacks

This adds two keywords to the language.

# Alternatives

# Unresolved questions
