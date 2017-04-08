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

## Bit-field access

Access through the `.` operator is unchecked. In other words, this is valid

```rust
fn f(node : KdNode) -> f32 { node.NodeX.axis }
```

If there is only one bit-constructor in the bit data, the constructor name may
be elided:
```rust
fn bus(pci : PCI) -> u8 { pci.bus }
```

## Matching

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

## Compared to `enum`

The `bitdata` type is similar to the existing `enum` type with the following
differences: 

* The discriminator is not added automatically. 
* All bit-data constructors must have the exact same bit-size.

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

This RFC does not discuss endianess issues. It is assumed that the bit-fields
are defined in target endianess.

Also, we could support inline-arrays of bit fields, but that could be saved 
for a future implementation. For instance:
```rust
bitdata KdTree {
   // ...
   Leaf  { tag = 3 : u2, _: u2, tri : [u20,..3] }
}
```
