- Start Date: 2014-10-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add unchecked indexing and slicing operators, accessible via 
```array[unsafe n]```.

# Motivation

Currently, unchecked indexing is among the safest unsafe operations 
in Rust, as the required invariant (in-boundness) is often satisfied 
locally (especially with Rust's aliasing rules, which prevent many of the 
usual ways for it to be violated), and even when it is not verifiable it
is desired.

Unfortunately, the current syntax for indexing is something like the ugly
```*a.get_unchecked(9)```, and if you're wrapping individual
operations in unsafe blocks it becomes even uglier
```unsafe { *a.get_unchecked(9) }```.

The ```s[unsafe n]``` seems light enough, while still warning users
of the Sword of Damocles that comes with unchecked indexing.

# Detailed design

Add 4 new operator traits, ```UnsafeIndex```, ```UnsafeSlice```,
```UnsafeIndexMut```, and ```UnsafeSliceMut```, with the following syntax:

* UnsafeIndex/UnsafeIndexMut
    ```Rust
    trait UnsafeIndex<E> {
        type R;
        unsafe fn index_unchecked<'a>(&'a self, element: &E) -> &'a R;
    }

    trait UnsafeIndexMut<E> {
        type R;
        unsafe fn index_unchecked_mut<'a>(&'a mut self,
	                                  element: &E) -> &'a mut R;
    }

    fn my_fn(s: &mut [uint]) {
        if s.len < 2 { return; }
        // ...
        s[unsafe 0] = f(s[unsafe 2]);
    }
    ```

* UnsafeSlice/UnsafeSliceMut
    ```Rust
    trait UnsafeSlice<Idx> {
        type S;
    	unsafe fn as_slice_unchecked<'a>(&'a self) -> &'a S;
        unsafe fn slice_from_unchecked<'a>(&'a self, from: Idx) -> &'a S;
        unsafe fn slice_to_unchecked<'a>(&'a self, to: Idx) -> &'a S;
        unsafe fn slice_unchecked<'a>(&'a self, from: Idx, to: Idx) -> &'a S;
    }

    trait UnsafeSliceMut<Idx> {
        type S;
        unsafe fn as_mut_slice_unchecked<'a>(&'a mut self) -> &'a mut S;
        unsafe fn slice_from_unchecked_mut<'a>(&'a mut self,
                                               from: Idx) -> &'a mut S;
        unsafe fn slice_to_unchecked_mut<'a>(&'a mut self,
                                             to: Idx) -> &'a mut S;
        unsafe fn slice_unchecked_mut<'a>(&'a mut self,
                                          from: Idx,to: Idx) -> &'a mut S;
    }

    fn my_ex_2(s: &mut [uint]) -> uint{
        if s.len < 5 { return; }
        // ...
        let v = calculate(s[unsafe 3..5]);
        // ...
        mutate(s[unsafe mut 0..1])
    }
    ```

Note that unsafe blocks aren't needed around unsafe indexing/slicing –
they "bring their own unsafe bubble" (but they don't
allow unsafe operations within the index, so
```s[unsafe ptr::read(ip)]``` *does* need an unsafe block).

The traits should be implemented for at least ```&T```, ```&mut T```, and 
```Vec<T>``` (of course, the ```&T``` does not need to implement the
```Mut``` ones). ```*const T``` and ```*mut T``` should implement the
slice traits, but I'm not sure they should implement the indexing traits,
as indexing unsafe slices involves some subtleties wrt. destructors
(however, the traits taking an ```&'a *T``` should ameliorate the
problem somewhat).

As a case study, here's an implementation of ```insertion_sort``` in terms
of the new functionality (other implementations can be seen in
RFC #365):

```Rust
/// Rotates a slice one element to the right,
/// moving the last element to the first one and all other elements one place
/// forward. (i.e., [0,1,2,3] -> [3,0,1,2])
fn rotate_right<T>(s: &mut [T]) {
    let len = s.len();
    let s = s.as_raw_mut();
    if len == 0 { return; }

    unsafe {
        let first = s.read(len-1);
        s[unsafe mut 1..].copy(s[unsafe ..len-1]);
        s.write(0, first);
    }
}

fn insertion_sort<T>(v: &mut [T], compare: |&T, &T| -> Ordering) {
    let len = v.len();

    // 1 <= i < len;
    for i in range(1, len) {
        // j satisfies: 0 <= j <= i;
        let mut j = i;

        // find where to insert, we need to do strict <,
        // rather than <=, to maintain stability.
        // 0 <= j - 1 < len, so j - 1 is in bounds.
        // and i is also in bounds
        while j > 0 && compare(v[unsafe i], v[unsafe j-1]) == Less {
            j-=1;
        }

        // `i` and `j` are in bounds, so [j, i+1) = [j, i] is valid.
        rotate_right(v[unsafe j..i-1]);
    }
}
```

# Drawbacks

A new operator set will add complexity to the language (there seems to be
a small combinatorical explosion with ```(Unsafe)?(Index|Slice)(Mut)?)```.
Significantly, there will be another syntax for unsafety, which
unsafety-scanners will need to notice.

In addition, the syntax would be slightly ambigious with non-block
unsafe expressions if they are ever introduced
(```a[unsafe ptr::read(x)]```). Giving this syntax precedence seems
to properly deal with it, especially because actually doing unsafe
operations will give you a clean compile-time error in this case.

# Alternatives

Enabling unchecked indexing via a crate or module attribute is a common
suggestion by game-developers. This has the unfortunate problem of
adding unsafety to all array accesses.

# Unresolved questions

Do we want direct unsafe indexing on raw slices? (Note that
*slicing* raw slices is completely fine and is suggested by this RFC).

Bikeshed: do we want to allow ```s[mut unsafe I]```, or only
```s[unsafe mut I]```