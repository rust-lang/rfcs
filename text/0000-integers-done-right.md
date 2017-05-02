- Feature Name: integers_done_right
- Start Date: 2014-04-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Finally solve the integer type problem with a flexible, yet simple system of type modifiers in the proven tradition of C, before it is too late for Rust 1.0.

# Motivation

There has been a lot of discussion about how to correctly handle integer types in Rust (e.g. [2, 5, 13, 23, 27]), most prominently whether to have types called `int` and `uint`, which either default to 32-bit numbers, 64-bit numbers or *pointer sized* numbers that are 32 bits when compiling for 32-bit systems and 64 bits when compiling for 64-bit systems.

Many good arguments have been brought forth for all of these options. On the one hand, 32 bits is enough in most situations, and defaulting to it will save space in many cases. On the other hand, integer overflow does happen ([7],[29]) and a 64-bit type will make this much less likely. On the third hand, using a pointer sized default integer type will allow for many important use cases to be handled correctly (indexing into large arrays, temporarily storing a raw pointer in an integer variable, ...) while not taking up more space than necessary on the given platform, albeit at the cost of potentially different behavior on different platforms.

It might seem, at a first glance, that this is a classical decision seen so often in programming language design, where there are trade-offs to be made along multiple dimensions: between safety and performance, between predictability and usability, maybe even between sensible contemplation and bold leadership. The detailed design I describe in the next section, however, will show that all along the discussion has been based on invalid preconditions. It will show that once the shape of things is laid out in front of us, all dispute ceases, all confusion evaporates. -- Please read on.


# Detailed design

I propose to combine the best of both Rust and C by introducing C's well-known and battle-tested `signed`, `unsigned`, `short`, and `long` integer type modifiers [17] *in addition* to keeping Rust's `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, and `u64` types. 

This allows for unprecedented flexibility and programmer control, sidestepping the pesky business of making trade-offs altogether. Similar to C, the modifiers affect whether the type is signed or unsigned and how many bits a value of the type occupies in memory. Not only will this make it easier for seasoned C/C++ programmers ([11]) to transition to Rust; together with Rust's pre-existing types, a whole new spectrum of possibilities emerges, as demonstrated in the following example:

```rust
/// Return the absolute of the maximum of two values
fn abs_max( a: signed long i32,
            b: signed long i32 )
            -> unsigned long u64 {
    // Take the greater of the two values
    let max = if a > b {
        a
    } else {
        b
    };
    
    // 3 bits is enough for a sign
    let sign: short i8 = signum( max );

    // the `long` modifier makes overflows highly unlikely
    let abs: unsigned long i32 = match sign {
        -1 => (max * -1) as unsigned long i32,
         0 => 0 as unsigned long i32,
         1 => max as unsigned long i32,
         _ => sqrt( -1 as signed short i32 ) as unsigned long i32
    };

    // let's be on the safe side though:
    // use the "escalator of safety" design pattern
    // to progressively widen the type to 129 bits
    return ((((abs as unsigned long u32)
                        as signed long i64)
                            as signed long u64)
                               as unsigned long u64);
}
```

The following table shows all possible integer types and their sizes in bits on different platforms:

```
 signed | default | short  |   long  |
--------------------------------------
   i8   |   8/8   |  4/4   |  16/16  |
  i16   |  16/16  |  8/8   |  32/32  |
  i32   |  32/32  | 16/16  |  64/64  |
  i64   |  64/64  | 32/32  | 128/128 |
  int   |  32/32  | 32/32  |  32/32  |
--------------------------------------
    u8  |   9/9   |  5/5   |  17/17  |
   u16  |  17/17  |  9/9   |  33/33  |
   u32  |  33/33  | 17/17  |  65/65  |
   u64  |  65/65  | 33/33  | 129/129 |
  uint  |  65/33  | 33/17  | 129/65  |


 unsigned | default | short  |   long  |
----------------------------------------
     i8   |   7/7   |  3/3   |  15/15  |
    i16   |  15/15  |  7/7   |  31/31  |
    i32   |  31/31  | 15/15  |  63/63  |
    i64   |  63/63  | 31/31  | 127/127 |
    int   |  32/32  | 32/32  |  32/32 |
----------------------------------------
     u8   |   8/8   |  4/4   |  16/16  |
    u16   |  16/16  |  8/8   |  32/32  |
    u32   |  32/32  | 16/16  |  64/64  |
    u64   |  64/64  | 32/32  | 128/128 |
   uint   |  64/32  | 64/32  |  64/128 |
```

Please observe that `unsigned` signed types like `unsigned i8` have one bit less, since they do not need to store their sign any more. This approach follows the Liskov substitution principle [19] since the programmer cannot expect an `i8` to store numbers greater than 127 anyway. Similarly, `signed` unsigned types like `signed long u64` need one bit more in order to store their sign. In the unlikely case that this leads to memory alignment issues, the recommended implementation strategy is to pad up to the next prime, or if need be, pool the additional bits in the cloud or on floppy disk. 

As you can see in the table, the problem of platform-dependent integer types is also solved rather elegantly: the `int` type is always 32 bits wide, regardless of the actual platform, while the `uint` type is 65/33/129 bits in the signed case and 64/64/64 bits in the unsigned case on 32-bit platform, and 33/17/65 bits signed, and 32/32/128 bits unsigned on 64-bit platforms, in order to avoid any confusion.

# Drawbacks

Not applicable.

# Alternatives

By now it should be obvious to the reader that there really is no alternative to this intricately refined, yet profoundly manly design. I have thoroughly tested all its possible implications in an experimental branch of my TempleOS port of Rust. Unfortunately, I cannot share this implementation for reasons I'd rather not discuss in public but I have no doubt that the community will step up and implement this before it is too late.

# Unresolved questions

Should this RFC be accepted and merged immediately?

# References

[2] https://github.com/rust-lang/rfcs/pull/464  
[3] https://www.wikipedia.org/  
[5] https://github.com/rust-lang/rfcs/pull/544  
[7] https://github.com/rust-lang/rust/issues/23115  
[11] http://www.stroustrup.com/  
[13] https://github.com/rust-lang/rfcs/pull/452  
[17] http://www.open-std.org/jtc1/sc22/wg14/  
[19] http://www.objectmentor.com/resources/articles/lsp.pdf  
[23] http://internals.rust-lang.org/t/implicit-widening-polymorphic-indexing-and-similar-ideas/  
[27] http://internals.rust-lang.org/t/call-for-consensus-if-we-do-want-rename-int-uint  
[29] https://github.com/pnkfelix/collab-docs/blob/master/rust/arith-overflow-buglist.md  
[31] https://www.youtube.com/watch?v=0-dVp542XGk  
