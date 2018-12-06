- Feature Name: `impl_trait_expressions`
- Start Date: 2018-12-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Rust closures allow the programmer to create values of anonymous types which
implement the `Fn*` traits. This RFC proposes a generalisation of this feature
to other traits. The syntax looks like this:

```rust
fn main() {
    let world = "world";
    let says_hello_world = impl fmt::Display {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "hello {}", world)
        }
    };

    println!("{}", says_hello_world);
}
```

# Motivation
[motivation]: #motivation

Sometimes we need to create a once-off value which implements some trait,
though having to explicitly declare a type in these situations can be
unnecessarily painful and noisy. Closures are a good example of how
this problem can be ameliorated by adding the ability to declare once-off
values of anonymous types.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`impl Trait` expressions allow the user to create a value of an anonymous type which
implements `Trait`. These expressions behave the same as closures - they
capture local variables, by either move or reference, and collect them into an
anonymous struct which implements the required trait.

To better understand the behaviour of `impl Trait`, consider the following code:

```rust
let y = String::from("hello");
let foo = move || println!("{}", y);
```

With this RFC, the above code becomes syntax sugar for:

```rust
let y = String::from("hello");
let foo = move impl FnOnce<()> {
    type Output = ();

    extern "rust-call" fn call_once(self, args: ()) {
        println!("{}", y);
    }
};
```

Which, in turn, is syntax sugar for:

```rust
let y = String::from("hello");

struct MyCoolAnonType {
    y: String,
}

impl FnOnce<()> for MyCoolAnonType {
    type Output = ();

    extern "rust-call" fn call_once(self, args: ()) {
        println!("{}", self.y);
    }
}

let foo = MyCoolAnonType { y };
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature is fully described in the guide-level explanation. As this is a
generalisation of the existing closure syntax I suspect that the implementation
would be fairly straight-forward.

# Drawbacks
[drawbacks]: #drawbacks

Adds yet another feature to an already-rich language.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Not do this.

# Prior art
[prior-art]: #prior-art

Other than closures I'm not aware of any prior art.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

It would be good if both closures and `impl Trait` expressions could implement
traits generically. For example we should be able to write:

```rust
let mut count = 0u32;
let print_numbered = move <T: Display> |val: T| {
    println!("{}: {}", count, val);
    count += 1;
};
```

Or, more verbosely:

```rust
let mut count = 0u32;
let print_numbered = impl<T: Display> FnOnce<(T,)> {
    type Output = ();

    extern "rust-call" fn call_once(self, (val,): (T,)) {
        println!("{}: {}", count, val);
        count += 1;
    }
};
```

To define a value that can be called with any `Display` type:

```rust
print_numbered(123);
print_numbered("hello");

// prints:
//  0: 123
//  1: hello
```

