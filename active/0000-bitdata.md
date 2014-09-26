- Start Date: 2014-07-03
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This RFC proposes to add support for bit-data types.

# Motivation

Rust aims to be a systems level language, yet it does not support bit-level
manipulations to a satisfactory level. We support a macro `bitflags!` for
supporting individual bits, but there is no support for bit-ranges. Anyone
who has had to write disassemblers for the x86 instruction set would concur
to how error-prone it is to deal with shifts and masks.

With this RFC accepted, we can describe a PCI address:

```rust
bitdata PCI {
    PCI { bus : u8, dev : u5, fun : u3 }
}
```

This definition describes a 16-bit value whose most significant eight bits
identify a particular hardware bus.

Immediate values can be specified anywhere in the definition, which provides 
a way to discriminate values:

```rust
bitdata KdNode : u64 {
    NodeX { axis = 0 : u2, left : u15, right: u15, split : f32 },
    NodeY { axis = 1 : u2, left : u15, right: u15, split : f32 },
    NodeZ { axis = 2 : u2, left : u15, right: u15, split : f32 },
    Leaf  { tag  = 3 : u2, _: u2, tri0 : u20, tri1 : u20, tri2 : u20 }
}
```
This defines a 64-bit value, where the two most significant bits indicate 
the type of node (internal node divided in x-, y- and z-axis, or a leaf 
node to the triangle vertex data).

With this in place, one could implement point lookup as such:
```rust
fn lookup(pt: Vec3, ns: &[KdNode]) -> Option<(uint,uint,uint)> {
   let mut i = 0u;
   loop {
     let n = match ns[i] {
       NodeX {left, right, split} => if pt.x < split { left } else { right },
       NodeY {left, right, split} => if pt.y < split { left } else { right },
       NodeZ {left, right, split} => if pt.z < split { left } else { right },
       Leaf  {tri0, tri1, tri2}   => return Some(tri0, tri1, tri2)
     };
     if n == 0 { return None }
     i = n;
   }
}
```

# Detailed design

All `bitdata` are calculated in units of bits instead of bytes. For this reason, 
it is illegal to take the address of individual components. 

## Syntax

The syntax needs to be extended with bit-sized integer literals. These are written
as `4u7`, or `-1i4`. In addition, bit-sized types of the form `u15` and `i9`
needs to be added. If the compiler needs to treat them as normal values,
zero- or sign-extension must take place.

```ebnf
BITDATA-DEFN      ::= "bitdata" IDENT (":" TYPE)? "{" BITDATA-CONS-LIST* "}"
```

This introduces the `bitdata` type with a name (the identifier), an optional
carrier type, followed by a block of bitdata constructors. The carrier type
is used as a substitution when regular data-types are needed. 

```ebnf
BITDATA-CONS-LIST ::= BITDATA-CONS ("," BITDATA-CONS)*
BITDATA-CONS      ::= IDENT "{" BITFIELD-LIST "}"
```

The bitdata constructors are all named constructors, each with bit-fields. A 
bit-field is either tag-bits or a labeled bit-field. Tag-bits are constant 
expressions (used for bit-field `match`), and labeled bit-fields are named
bit-ranges.

```ebnf
BITFIELD-LIST     ::= BITFIELD ("," BITFIELD)*
BITFIELD          ::= TAG-BITS | LABELED-BIT-FIELD
TAG-BITS          ::= BIT-LITERAL
LABELED-BIT-FIELD ::= IDENT ( "=" CONST-EXPR )? ":" BITDATA-TYPE
```

The valid bitdata-types are only other bitdata-types (by name) or else unsigned
and signed bit-types like e.g. `u12`, and also floating-point value types.

```ebnf
BITDATA-TYPE      ::= ("u" | "i") ('0'-'9')+ | "f32" | "f64" | IDENT
BIT-LITERAL       ::= INT-LITERAL ("u" | "i") ('0'..'9')+
```

## Limitations

* Each constructor must have the exact same bit-size. 
* If the `bitdata` definition has a type specifier, all constructor bit-sizes must match this.
* Tag-bits and labeled bit-fields with initializers act as discriminators, but they need
not be exhaustive.

## Construction

```rust
  let addr = PCI { bus : 0, dev : 2, fun : 3 };
  let tree = vec![ NodeX { left: 1, right: 2, split: 10.0 }, // 0
                   NodeY { left: 3, right: 0, split: 0.5 },  // 1
                   Leaf { tri0: 0, tri1: 1, tri2: 2 },       // 2
                   Leaf { tri0: 3, tri1: 4, tri2: 5 } ]      // 3
```

## Bit-data access

There are two ways of getting access to bit-data; by using `match` (bit-data 
patterns) or by bit-field access.

### Matching

Matching is not nescessarily exhaustive, as there may be "junk" values. For
instance, 
```rust
bitdata T { S { 0u5 }, N { 0b11111u5 } }
```
Here `T` is 5-bits, but if the value is anything else than 0 or 31, it is
considered "junk":
```rust
match t { S => "Zero", N => "Non-zero", _ => "Junk" }
```

While enums use exhaustiveness checks to ensure safety, bit-data matches may
not. Instead, a match is implemented (at least semantically) through a series 
of `if`-`else` tests. Warnings should however be generated if unreachable 
match-arms exist. For instance, if the first pattern does not have any 
discriminant bits (tag bits) set, then the first match arm will always be
taken:

```rust
bitdata U { 
  A { data : u32 },
  B { 0u2, rest : u30 }
}

fn test(u : U) -> u32 {
  match u { 
    A {data} => data, // Always taken.
    B {rest} => rest  // Warning: Unused match arm
  }
}
```

### Bit-field access

Bit-data variants can be unwieldly for enum variants with many fields. For 
regular enums, we solve the problem by using named fields -- in other words, 
we create structs. Notice that unlike regular enums, bit-data variants are 
already named, hence we should be able to get access to a bit-field using 
the bit-field name.

```rust
fn axis(node : KdNode) -> u32 { node.NodeX.axis }
```

Notice that bit-field access is unchecked, so `axis(node)` would result 
in `3` if `node` is a leaf. While this feature may seem unsafe, it is 
a useful construct when the value depends on external data. For instance:

```rust
bitdata PortResult : u32 {
    E { code : u16, src : u16 }, // Error
    F { f : f32 },
    U { u : u32 }
}

fn parse_port_result(signal: u32, result: PortResult} {
  match signal {
    0 => println!("Error: {}, {}", result.E.code, result.E.src),
    1 => println!("Float: {}", result.F.f),
    2 => println!("Int: {}", result.U.u),
    _ => println!("Invalid port result")
  }
}
```
Here, hardware gives us a result as a 32-bit value, but the value can't 
be discriminated by the value itself.

If there is only one bit-constructor in the bit data, or if all bit-data
variants place the same bit-field in the same bit location, then the 
constructor name may be elided:

```rust
fn bus(pci : PCI) -> u8 { pci.bus }
```

## Byte Order

Byte order is not defined for bit-data. This makes sense since bit-data is 
simply defined in terms of bit-positions within an unsigned integer. Storage
of the integer could be specified in either big-endian or little-endian 
formats, but that is outside the scope of this RFC.

## Bit Order

While bit-data does not define byte-order, sometimes it is useful to specify
the bit-order. By default it is most significant bits first. If this is to
be changed it has to be done through an attribute, like the equivalent 
definition below:

```rust
#[bitorder(lsf)]
bitdata PCI {
    PCI { fun : u3, dev : u5, bus : u8 }
}
```

## Compared to `enum`

The `bitdata` type is similar to the existing `enum` type with the following
differences: 

* The discriminator is not added automatically. 
* All bit-data constructors must have the exact same bit-size.
* Exhaustiveness checks are more forgiving.

## Notes

`bitdata` may help reduce some unsafe operations such as transmute. For instance,
we can analyse a IEEE-754 value:

```rust
bitdata IEEE754 {
   F { value : f32 },
   I { sign : u1, exp: u8, mant: u23 }
}

fn float_rep(f : f32) {
  let x = F { value : f };
  println!("s:{}, e:{}, m:{}", x.I.sign, x.I.exp, x.I.mant)
}
```

### Byte data

The carrier type is typically a `u8`, `u16`, `u32`, `u64`, etc., but it
can also be an array type:

```rust
bitdata PackedRgb : [u8, ..3]
{
  RGB { r: u8, g: u8, b: u8 }
}
```

# Alternatives

It has been suggested to implement this a syntax extension. This will not 
work, because

* We need significant error-checking, including bit-size calulations
and overlapping tag checks
* `bitdata` definitions may make use of other `bitdata` definitions
* Syntactic overhead would be large
* It is unclear how cross-module usage and type-checking would occur

# Drawbacks

# Unresolved questions

## Inline Arrays

We could support inline-arrays of bit fields, but that could be saved 
for a future implementation. For instance:

```rust
bitdata KdTree {
   // ...
   Leaf  { tag = 3 : u2, _: u2, tri : [u20,..3] }
}
```

