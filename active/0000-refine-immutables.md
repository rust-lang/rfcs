- Start Date: 2014-09-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

This RFC proposes that the semantics of immutable variables be refined by forbidding partial outbound moves, so:

1. immutable variables in Rust become more immutable;
2. guaranteed scoped lifetimes for values with move semantics ("movable values") can be achieved.

# Motivation

This RFC is motivated by two problems in Rust today:

## "Immutable" variables are not immutable enough.

Rust's "immutable" variables do *not* provide *strict immutability*. There are three exceptions:

1. it is legal to have internal mutability even inside "immutable" variables, via `UnsafeSell<T>`;
2. it is legal to move values from "immutable" variables as a whole;
3. it is legal to move parts of compound values from "immutable" variables.

So, "immutable" is not exactly accurate, but is it good *enough*?

Exception 1 can be justified because well, `UnsafeCell<T>` *is* unsafe.
Exception 2 can be justified because the only thing that changes after a *full outbound move* is the value's location, not the value itself.

But there is a problem: Exception 3 is very hard to justify, as after a *partial outbound move*, the value itself is mutated.

Consider the following snippet:

```rust
#[deriving(Show)]
enum Gender { Male, Female }

#[deriving(Show)]
struct Person {
    name: String,
	gender: Gender,
}

fn main() {
    // not supposed to change:
    let person = Person { name: "Mike".to_string(), gender: Male };

    // person.name = "Clark";  // compile error
    // person.gender = Female; // compile error

    // seemed innocent:
    match person {
        Person { name: n, gender: Male } => println!("I am {}, a man!", n),
        Person { name: n, gender: Female } => println!("I am {}, a woman.", n),
    }
    
    // but the name was moved:
    // println!("Oh yes, I am {}!", person.name); // compile error
    
    // the value as a whole, was rendered unusable:
    // println!("Hey! I am {}!", person); // compile error
    
    // though the other part can still be used:
    println!("What?! I am still a {}!", person.gender);
}
```

This snippet compiles and runs - and that's the problem.

There were partial outbound moves happening in the match arms, which was unexpected for two reasons:

1. `person` was supposed to be *immutable*;
2. even if the programmer knew that `person` is not strictly immutable, it was likely that he/she may not expect the innocent looking matches to move his name - he should have used `ref n`, not bare `n`.

So, forbidding partial moves from immutable variables can have the following benefits:

1. the semantics of immutable variables will be less against programmer intuitions;
2. certain kinds of unexpected moves will be prevented on spot.

## Guaranteed lifetimes for movable values

In [RFC PR 210: Static drop semantics](https://github.com/rust-lang/rfcs/pull/210), `NoisyDrop`/`QuietDrop` are proposed to help programmers identify unwanted implicit drops introduced by the new semantics. The reason that some so-called "early" drops (or "implicit balancing drops") are unwanted is because the programmer, for whatever reason, wants to ensure that some movable values have certain guaranteed lifetimes.

`NoisyDrop`/`QuietDrop` can help, but:

1. the necessity of guaranteed lifetimes depends heavily on context, and should be decided by the application programmer on a case-by-case basis, not by library types;
2. for all their troubles, `NoisyDrop`/`QuietDrop` still cannot guard against all the possibilities that would threaten the guarantee.

Because moves transfer ownership and lifetime control to the receiver, and the original owner can guarantee nothing once ownership is transferred. This is true regardless of where the moves happen or if they are balanced or not, or if they are implicit or not (a drop can be seen as a special kind of move - a move into oblivion).

Therefore:

**The only way to guarantee that a value has a certain lifetime is to maintain ownership and do not move it before the intended drop point.**

The above is true regardless of which drop semantics is used. Dynamic drops? Static drops? Even eager drops? Doesn't matter.

Then, how can this be done?

It is actually quite simple: if guaranteed lifetime is necessary, then explicitly call `drop` at the intended drop point.

Because if unexpected moves happen before the explicit drop, a compile error is guaranteed happen, though the compiler will complain about the explicit drop, not the unexpected moves. However in practice this is not a problem.

Let's call this *value pinning*.

There is still a problem with this form of pinning, as it is *shallow* in that only the *root* value is pinned, but partial moves from the value is still allowed. Shallow pinning cannot guard against the possibility of losing lifetime control of parts of a compound value.

That's when our refined immutable variables come into play:

By combining explicit drops and refined immutable variables, *deep* pinning and truly guaranteed lifetime for movable values can be achieved.

# Detailed Design

To forbid partial outbound moves from immutable variables, only one rule is needed to be added to Rust's mutation control semantics:

* Outbound moves from inherently immutable struct fields or enum variant fields are forbidden.

After the change, a programmer will be required to use mutable variables if he/she truly wants partial outbound moves.

# Drawbacks

Breaking change. `mut`s and `ref`s may have to be added.

The RFC author (@CloudiDust) believes this to be generally a plus. Rust prefers doing things (reasonably) explicitly after all. Partial outbound moves are mutations to the parent values, and should have been labelled as such. Currently they just slip past the radar.

The problem with using more `mut` is that: `mut` dictates no mutation restrictions at all, and it is not clear what kind of mutation the programmer actually wants when he/she writes `mut`. Inbound copies? Or inbound moves? Or outbound moves? Fully or partially?

Note this problem already exists in Rust today.

Actually, immutable and mutable variables each codify a commonly used mutation control policy, bBut there are many more. It may be beneficial to support more fine grained mutation control, but this is outside the scope of this RFC. The RFC author already has some ideas on the design for that feature, the design is backwards-compatible (other than that it requires adding a `pin` or `pinned` keyword), so it can be postponed, but it relies on this RFC being accepted.

# Alternatives

Alternative 1. Instead of forbidding partial moves from immutable variables, forbid reading any remaining part of a partially moved value (either mutable or immutable), but still allow the remaining parts to be moved.

This was suggested when this RFC was still a pre-RFC. This new rule would help finding out unexpected partial outbound moves in some cases. However, a value that can be moved but not read doesn't make sense.
 
Alternative 2. Maintain the status quo.

It can be argued that, because reading partially-moved values (as a whole) or empty slots are compile errors, many of the bugs caused by unexpected partial moves from immutable variables will "eventually" be caught, so the status quo may not be *that* bad in practice. But it is still better to catch more bugs on spot, and calling partially moved values "not mutated", is hardly justifiable. Also there will be no way to deeply pin a value.

Alternative 3. Go all the way and forbid full outbound moves from immutable variables as well.

This would make immutable variables even more true to their names, and effectively this is deeply pinning all immutable values. But this is too restrictive and unnecessary. Having to throw `mut` everywhere defeats the purpose of the mutable/immutable distinction. Also, unexpected full outbound moves are easier to catch than unexpected partial outbound moves as compile errors will happen more often.

The RFC author considers the proposed change in this RFC to be a reasonable compromise between the alternatives 2 and 3. 

# Unsolved Questions

None.
