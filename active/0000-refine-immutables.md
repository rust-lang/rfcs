- Start Date: 2014-09-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

This RFC proposes that the semantics of immutable variables be refined by forbidding partial outbound moves, so immutable variables in Rust become more immutable.

# Motivation

This RFC is motivated by the following problem in Rust today:

## "Immutable" variables are not immutable enough.

Rust's "immutable" variables do *not* provide *strict immutability*. There are three exceptions:

1. it is legal to have internal mutability even inside "immutable" variables, via `UnsafeCell<T>` and the likes;
2. it is legal to move values from "immutable" variables as a whole;
3. it is legal to move parts of compound values from "immutable" variables.

So, "immutable" is not exactly accurate, but is it good *enough*?

Exception 1 can be justified because well, `UnsafeCell<T>` and the likes are explicitly designed for *internal* mutability, and must be opted-in by the programmer. (Also, see the discussion for Alternative 2 below, which explains this one further.)

Exception 2 can be justified because the only thing that changes after a *full outbound move* is the value's location, not the value itself.

But there is a problem: Exception 3 is very hard to justify, as after a *partial outbound move*, the value itself is mutated.

Consider the following snippet:

```rust
#[deriving(Show)]
enum Status { Living, Deceased }

#[deriving(Show)]
struct Person {
    name: String,
    status: Status,
}

fn main() {
    // not supposed to change:
    let person = Person { name: "Mike".to_string(), status: Living};

    // person.name = "Clark";  // compile error
    // person.status = Deceased; // compile error

    // seemed innocent:
    match person {
        Person { name: n, status: Living } => println!("{} is alive.", n),
        Person { name: n, status: Deceased } => println!("{} is dead.", n),
    }
    
    // but the name was moved:
    // println!("Hi, {}!", person.name); // compile error
    
    // the value as a whole, was rendered unusable:
    // println!("{}", person); // compile error
    
    // though the other part can still be used:
    println!("The nameless person is still {}.", person.status);
}
```

This snippet compiles and runs - and that's the problem.

There were partial outbound moves happening in the match arms, which was unexpected for two reasons:

1. `person` was supposed to be *immutable*;
2. even if the programmer knew that `person` is not strictly immutable, it was likely that he/she may not expect the innocent looking matches to move the name - he/she should have used `ref n`, not bare `n`.

So, forbidding partial moves from immutable variables can have the following benefits:

1. the semantics of immutable variables will be less against programmer intuitions;
2. certain kinds of unexpected moves will be prevented on spot.

(Here used to be a section about how this RFC will help providing guaranteed lifetimes. But it turns out that explicit drops *alone* can achieve that. Please refer to the comments of [RFC PR 210](https://github.com/rust-lang/rfcs/pull/210) for more details.)

# Detailed Design

To forbid partial outbound moves from immutable variables, only one rule is needed to be added to Rust's mutation control semantics:

**Outbound moves from inherently immutable struct fields or enum variant fields are forbidden.**

After this change, a programmer will be required to use mutable variables if he/she truly wants partial outbound moves.

# Drawbacks

Breaking change. `mut`s and `ref`s may have to be used in more places.

Particularly, if for some reason, partial outbound moves from immutable values are intentionally requested by a programmer, then he/she has to use this:

```rust
let foo = Foo {...};
...
let mut foo = foo;
move(foo.bar);
...
```

Instead of simply:

```rust
let foo = Foo {...};
...
move(foo.bar);
...
```

There may be some ergonomic issues, and the programmer can no longer move `foo` back into an immutable variable. (Arguably this can be a good thing, as immutable values will be guaranteed to be without "holes".) The RFC author (@CloudiDust) believes this change to be generally a plus. Rust prefers doing things (reasonably) explicitly after all. Partial outbound moves are mutations to the parent values, and should have been labelled as such. Currently they just slip past the radar.

The problem with using more `mut` is that: `mut` dictates no mutation restrictions at all, and it is not clear what kind of mutation the programmer actually wants when he/she writes `mut`. Inbound copies? Or inbound moves? Or outbound moves? Fully or partially?

Note this problem *already exists* in Rust today.

Actually, immutable and mutable variables each codify a commonly used mutation control policy, but there are many other possibilities. It may be beneficial to support more fine grained mutation control, but this is outside the scope of this RFC. The RFC author already has some ideas on the design for that feature, *which also solves the ergonomic issues mentioned above*. The design is backwards-compatible (other than that it requires adding a `pin` or `pinned` keyword), so it can be postponed, but it relies on this RFC being accepted.

# Alternatives

**Alternative 1.** Instead of forbidding partial moves from immutable variables, forbid reading any remaining part of a partially moved value (either mutable or immutable), but still allow the remaining parts to be moved.

This was suggested when this RFC was still a pre-RFC. This new rule would help finding out unexpected partial outbound moves in some cases. However, a value that can be moved but not read doesn't make sense.
 
**Alternative 2.** Maintain the status quo.

It can be argued that, because using partially-moved values (as a whole) or empty slots are compile errors, many of the bugs caused by unexpected partial moves from immutable variables will eventually be caught. Also, types implementing `Drop` are *always* not partially-movable already, so the status quo may not be *that* bad in practice.

Also, there is a reason that Rust's mutation control semantics are designed the way it is. `mut`/`&mut` are not actually designed around *mutability*, but *exclusive accessibility*. What Rust has is not *mutation control* but *uniqueness and aliasing control*, and `mut` actually means "this is a variable that you can request exclusive access of", not *necessary* "this is a variable that is mutable". So conversely, a variable without the `mut` keyword, is just a variable that "you cannot (statically) request exclusive access of", not *necessary* "a variable that is immutable". (Exclusive/mutable access to immutable values can be requested and checked dynamically with the `UnsafeCell<T>` family of types.) 

But it is still better to catch more bugs on spot, and calling partially moved values "not mutated", is hardly justifiable. The RFC author believes that, uniqueness and aliasing control is but an implementation detail. If the keyword is called `mut`, then "mutable" and "immutable" should fit programmer intuitions.

**Alternative 3.** Go all the way and forbid full outbound moves from immutable variables as well.

This would make immutable variables even more true to their names, and effectively this would deeply pin all immutable values and guarantee that they all have scoped lifetimes. But this is too restrictive and unnecessary. Having to throw `mut` everywhere defeats the purpose of the mutable/immutable distinction.

The RFC author considers the proposed change in this RFC to be a reasonable compromise between the alternatives 2 and 3. 

# Unsolved Questions

None.
