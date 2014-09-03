- Start Date: 2014-09-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make [std::num::Int][1]'s `leading_zeros()`, `trailing_zeros()`, `count_ones()` and `count_zeros()` return `uint` instead of `Self`.

# Motivation

These 4 methods return numbers related to number of bits, which are represented as `uint` in other places like `<<`, `>>` and `::BITS`. The change makes these methods consistent with the other features.

Practically, since leading_zeros and trailing_zeros are often involved in shifting, it is better to return a `uint`. [Example 1][2]:

```rust
fn get_size_class(size: u32) -> u32 {
    let pow2 = 1 << (32 - (size - 1).leading_zeros()) as uint;
//                                                    ^^^^^^^
    if pow2 < 16 { 16 } else { pow2 }
}
```

[Example 2][3]:

```rust
let nlo = (!first).leading_zeros();
...
let first_mask = (1 << (7 - nlo) as uint) - 1;
//                               ^^^^^^^
```

The standard library's `BigUint::bits()` also [casts the result][2] to `uint`, even though it is because `bits()` itself returns a `uint`:

```rust
let zeros = self.data.last().unwrap().leading_zeros();
return self.data.len()*BigDigit::bits - (zeros as uint);
//                                             ^^^^^^^
```

# Detailed design

Change the methods of the trait std::num::Int to:

```rust
fn count_ones(self) -> uint;
fn count_zeros(self) -> uint { (!self).count_ones() }
fn leading_zeros(self) -> uint;
fn trailing_zeros(self) -> uint;
```

If possible, also change the signatures of these intrinsics (std::intrinsics):

```rust
pub fn ctlz8(u8) -> uint;
pub fn ctlz16(u16) -> uint;
pub fn ctlz32(u32) -> uint;
pub fn ctlz64(u64) -> uint;
pub fn cttz8(u8) -> uint;
pub fn cttz16(u16) -> uint;
pub fn cttz32(u32) -> uint;
pub fn cttz64(u64) -> uint;
pub fn ctpop8(u8) -> uint;
pub fn ctpop16(u16) -> uint;
pub fn ctpop32(u32) -> uint;
pub fn ctpop64(u64) -> uint;
```

# Drawbacks

* Code which relies on these methods returning Self needs to be modified.
* The return type becomes platform-dependent, while we are moving away from `int` and `uint`.

# Alternatives

* **Keep the status quo**. Since most of the existing code can already solve the issue with `as uint`.

* **Make `<<`, `>>` etc take `Self`**. But this would introduce more trouble, e.g. shifting with negative number in signed types.

# Unresolved questions

None




[1]: http://doc.rust-lang.org/std/num/trait.Int.html
[2]: https://github.com/thestinger/rust-alloc/blob/ac0e693262b8dbe5ce606b890cc49f7fbc917b3c/allocator.rs#L116-119
[3]: https://github.com/tari/rust-flac/blob/e27502abb8e51aa120f15e0215cd5fe4c183dae0/src/bitstream.rs#L120-138