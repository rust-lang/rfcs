- Start Date: 2014-05-20
- RFC PR #:
- Rust Issue #:

# Summary

Add syntax to partially destructure `self` in method signatures such that method calls are possible even if parts of the object are borrowed.

# Motivation

Consider the following struct:
```rust
struct X {
    a: Vec<Y>,
    b1: T1,
    b2: T2,
    b3: T3,
}

impl X {
    fn f1(&mut self) {
        for obj in self.a.mut_iter() {
            some_function(obj);
            self.g(obj);
        }
    }

    fn f2(&mut self) {
        for obj in self.a.mut_iter() {
            another_function(obj);
            self.g(obj);
        }
    }

    fn g(&mut self, obj: &mut Y) {
        /* long function that doesn't access `self.a` */
    }
}
```
This is currently not possible because `self.g` wants to borrow `self` which is already partially borrowed.
The syntax I propose makes this possible.

# Drawbacks

The syntax is not very pretty.

A method that uses this syntax cannot call other methods.

This is a special case for `self` and methods.

# Detailed design

The new `g` would look like this:
```
    fn f1(&mut self) {
        /* ... */
            self.g(obj);
        /* ... */
    }

    fn g(&mut {b1, b2, b3}, obj: &mut Y) {
        /* long function */
    }
```
This would be effectively equivalent to this function:
```
    fn f1(&mut self) {
        /* ... */
            X::g(&mut self.b1, &mut self.b2, &mut self.b3, obj);
        /* ... */
    }

    fn g(b1: &mut T1, b2: &mut T2, b3: &mut T3, obj: &mut Y) {
        /* long function */
    }
```

# Alternatives

### 1

The borrow checker could be made smart enough to detect that `self.a` isn't used in `g`.

### 2

It is already possible to do this by turning `g` into a non-method and calling `g(&mut self.b1, ..., obj)`.
But if the names are descriptive and `g` uses many fields, this gets ugly.
It is also unnatural for `g` not to be a method since it only accesses fields.

### 3

Instead of iterating over the vector itself, one could do the following:
```rust
    fn f2(&mut self) {
        for i in range(0, self.a.len()) {
            another_function(self.a.get_mut(i));
            self.g(i);
        }
    }

    fn g(&mut self, i: uint) {
        let obj = self.a.get_mut(i);
        /* ... */
    }
```
This is in some ways equivalent to passing a raw pointer.
Inside `g` there are no guarantees that `i` points to a valid element or that it is the element the caller wanted to pass.

This approach makes `g` less generic.
One might want to call `g` on a `&mut Y` that is not in `self.a` or `self` might contain two `Vec<Y>`.

Furthermore, consider the case where `self.a` is a hashmap.
Passing the key instead of the value entails another expensive lookup.

### 4

One might change the definition of `X`:
```rust
struct Bs {
    b1: T1,
    b2: T2,
    b3: T3,
}

struct X {
    a: Vec<Y>,
    bs: Bs,
}
```
This is unnatural in many cases and there are many possible ways to partition `X`.

# Unresolved questions

The exact syntax.
Consider
```rust
    fn g(&mut self{b1, b2, b3}, obj: &mut Y) {
    }
    fn g(&mut {ref b1, ref b2, ref b3}, obj: &mut Y) {
    }
    fn g(&mut {ref mut b1, ref mut b2, ref mut b3}, obj: &mut Y) {
    }
```
