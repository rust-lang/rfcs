
- Feature Name: Legal Double Reference
- Start Date: 2017-12-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

The Rust borrow checker does not allow you to make an immutable reference to something while a mutable reference exists. The reasons for this are made clear, however its implementation is too strict; I propose that an immutable reference to something should be allowed to be created while there is an existing mutable reference if it is dropped before the next use of the mutable reference.  
This change will mean that getting useful values from types while mutating them will be less difficult and encourage functional programming in Rust while preserving its safety.

# Motivation
[motivation]: #motivation

This change should be made because it encourages use of existing functional methods in Rust which can sometimes be difficult to use.  
The use cases of this change are times when a value is being mutated but during that time some method needs to be called which requires an immutable reference to the value.
```rust
let mut a = vec![0u32; 3];

a.iter_mut()
.for_each(|b| {
    //Do stuff with b.
    let c = a.len(); //E0502
    //Do more stuff with b.
});
```
By making this change functional programming in Rust will be easier to do and unnesting will be less necessery to write Rust code.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

In `Safe` Rust code it is true that between uses of a mutable reference `&mut _` the value referenced will not change i.e.
```rust
let mut a = vec![0u32; 3];
let b = &mut a;
//Do stuff with b.
let c = a.len(); //Uses immutable reference which is created an dropped between uses
        //of the mutable reference to `a`. `a` is guarenteed not to change in this time.
//Do more stuff with b.
```
However the Rust compiler does not allow this because there is an existing mutable reference to `a` created by the call `.iter_mut()`. This change does not invalidate Rust's borrow safety but it will make references and all types with pure functions easier to use in Rust which will make writing Rust code more straight forward especially for users new to Rust since the programmer will not have to work around the borrow checker when writing safe logic similar to the example above.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This change only impacts the Rust borrow checker and should have no impact on other features of the language.
An implementation this feature, as best as I can determine, would be as such:
1. The `borrow checker` identifies the creation of an immutable reference while there is an existing mutable reference as it already does.
2. The `borrow checker` confirms that the immutable reference is dropped before the next use of the mutable reference (in the case of a function, this may require extra logic to confirm that the mutable reference is also not a parameter of the function).
3. The `borrow checker` throws the existing `E0502` if the aformentioned check fails.

At this time I do not believe there are any corner cases in this proposed implementation.

# Drawbacks
[drawbacks]: #drawbacks

The absence of this feature in the language does not make logic such as the above example impossible, mearly requiring that the programmer unnests the references so that the immutable reference is used and dropped before the mutable one is created i.e.
```rust
let mut a = vec![0usize];

//a.push(a.len()); //Does not compile.

let b = a.len();
a.push(b);
```

# Rationale and alternatives
[alternatives]: #alternatives

The current unnesting required to write the kind of logic shown in this RFC requires the programmer to do one of:
* Know what values they will need before actually writing the code.
* Keep Rust's borrow checker in mind while writing their code.
* Go back to create a new unnested value each time they need to create a new immutable reference while there is an existing mutable reference.

Each of these options either require the programmer to hold the borrow checker in mind or break their workflow to work around the borrow checker.  
Another alternative would be to remove the Rust borrow checker but this would go against the principals of Rust.  
The proposed change preserves Rust's safety and principals and prevents the programmer having to work around the borrow checker.  
Not making this change means that the programmers workflow will often be interupted while writing Rust to unnest their calls and programmers new to Rust will have to encounter the strange and unituitive E0502 error while writing code that it is reasonable to expect to work in Safe Rust.

# Unresolved questions
[unresolved]: #unresolved-questions

During the RFC process I expect:
* The specific implementation of this change to be discussed.
* Any unidentified edge cases of this change to be identified and discussed.
