- Feature Name: unsafe_modules
- Start Date: 2017-09-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
Allow unsafe at more positions.

# Motivation

When writing unsafe code it's annoying having to write unsafe everywhere. Normally that's a good thing, but sometimes, especially for prototyping, it would sometimes be preferable to be able to declare unsafe for larger blocks.
Maybe it's also helpful for low level code with huge amount of unsafe functions or c interaction.

# Guide-level explanation

Unsafe blocks can also be written outside of functions:
```
unsafe {
    fn x() {...}
    trait X {...}
    ...
}
```

This will make the elements defined inside the block unsafe.
It will be equivalent to following code:
```
unsafe fn x() {...}
unsafe trait X {...}
unsafe ...
```

Even modules can be declared as unsafe like this:
```
unsafe mod test {
    fn x() {...}
    trait X {...}
    ...
}
```

This is almost equivalent to following: 
```
mod test {
    unsafe {
        fn x() {...}
        trait X {...}
        ...
    }
}
```
But unsafe modules have an additional property.
Using such a module is also unsafe:
```
unsafe use test;
fn main() {
    unsafe {
        test::x();
    }
}
```



# Reference-level explanation
This could just be implemented by some syntactic transformations.

# Drawbacks
The use of unsafe may be encouraged.

# Rationale and Alternatives
It may also be possible to add a compiler option to allow unsafe code without declaration. But this option should offer more control about, what is unsafe.

# Unresolved questions
Should `use` really be `unsafe`?
If it's unsafe to use the module, should calling the functions still be unsafe or will an additional `unsafe` block be needed for calling?
Should whole crates or other things also be able to be declared as unsafe?
How should elements inside the block be handled, if they cannot be declared as unsafe (like variables).
If unsafe fields will be implemented, will it be still required to declare them unsafe explicitely?
