- Start Date: 2014-08-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

The problem:

Rust's current `unsafe` blocks are a two-part construct:  

1. They allow the programmer to perform unsafe operations.  
2. They imply the operations are safe in the surrounding context, where the context can be as small as the block itself or as large as the whole module (or the complete set of methods on a struct). The context is determined by the programmer, is often unclear, and no documentation of it is enforced.

My claim is that the current model for unsafe blocks complicates the use of unsafe code, and makes it dangerously and unnecessarily easy to punch holes in a program's safety guarantees through misuse.

The proposed solution:

Require the programmer to explicitly declare where a safe interface exists. This declaration will happen at the function level. The most important components of this proposal are:  

1. Mandate that functions containing unsafe blocks be marked as `unsafe` by default.  
2. Introduce a `#[safe_interface]` attribute which permits the exclusion of the `unsafe` qualifier on functions in exchange for programmer assurance that a function is safe to execute for all inputs.

# Motivation

The goal is to force the programmer to explicitly declare (and think about) their intent when they write unsafe code, and have it correctly documented for future maintainers. Currently, when you put an unsafe block in a safe function the rule is "assume the function is still safe" by default. Under the proposed new rules, it becomes "error by default; have the programmer choose explicitly between safe and unsafe". 

The programmer will need to make a conscious decision about how the unsafe code should be treated, since they can no longer just place an unsafe block down and forget about it. If the function is not safe to execute for all inputs, then the programmer should mark the function as `unsafe`. If the programmer believes the function exposes a safe interface, then they can proclaim a `#[safe_interface]`. If they make neither of these choices, the program won't compile. The compiler makes no assumptions about safety without written permission.

`#[safe_interface]` is treated as a safety override, rather than a normal language feature. It will be the point where safe code and unsafe code are stitched together, and the point where the safety leaks occur. Another name like `#[override(safe)]` could be used to emphasise this point, but we'll stick with `#[safe_interface]` for the rest of this RFC.

# Detailed design

This RFC draws on previous discussion of unsafe blocks:  
https://github.com/rust-lang/rfcs/pull/117    
http://huonw.github.io/2014/07/24/what-does-rusts-unsafe-mean.html

And unsafe fields:  
https://github.com/rust-lang/rfcs/pull/80    
https://github.com/rust-lang/meeting-minutes/blob/master/weekly-meetings/2014-06-17.md#rfc-pr-80-unsafe-fields-httpsgithubcomrust-langrfcspull80-

There are 4 main changes proposed:

1. Functions containing unsafe blocks must be marked as `unsafe`, otherwise the compiler gives an error.  
2. Marking a function with the `#[safe_interface]` attribute overrides this requirement.  
3. Unsafe functions no longer permit unsafe code anywhere. For clarity, the unsafe portion of the code must still be enclosed in an unsafe block.  
4. Struct fields can be marked as `unsafe`, preventing them from being modified in safe code. This is to allow unsafe functions to impose and assume invariants on objects, the canonical example being `Vec` and its length. While this may seem tangential to the rest of the proposal, it's an important part of plugging the holes between safe and unsafe code.

The functions and responsibilities of the important constructs are as follows:

Unsafe block: Unsafe operations such as raw pointer dereferencing and unsafe function invocation are permitted within this block. Execution of an unsafe block is not guaranteed to be safe in all contexts.

Unsafe function: A function that is not safe to execute for all inputs. It must be called from within an unsafe block.

`#[safe_interface]`: An annotation that permits compilation of an unsafe function without the unsafe keyword. The programmer makes the guarantee that in practice, the annotated function is safe to execute for any input generated from safe Rust code, and likewise, the output is safe to use in safe Rust code.

Unsafe field: A struct field that can only be modified in an unsafe block. It can still be read by safe code, if visibility permits.

I'll explain the rationale behind unsafe fields a little more.

In today's Rust, a function (as part of an actively-changing program) has no way to know who is calling it and what values are being passed in, so to be considered a safe function they must be safe to execute for all inputs. Safe code can initialize any struct to any state as long as the fields are visible to the code*. But sometimes we need to impose invariants or restrictions on the state of a struct to maintain safe behaviour. Unsafe fields allow us to achieve this, even within a module where there are no privacy barriers. They also add to the self-documentation of the struct, communicating the danger of modifying the fields.

\* An argument can be made that a function can hide things with private fields, but using this as a protection mechanism still leaves safety holes within the module. While it might be expected that a module-writer understands the scope of the module and what invariants need to be upheld, the maintainers who come along later might not be as familiar with them. I'm of the view that it's better to be safe than sorry in this situation.

# Examples

```rust
// The compiler throws an error because the function performs unsafe
// operations outside an unsafe block.
unsafe fn foo() {
    // unsafe operations
}
```
```rust
// The compiler accepts this code since the unsafe operations are
// contained within an unsafe block and the function is marked as unsafe.
unsafe fn foo() {
    // safe operations
    unsafe {
        // unsafe operations
    }
    // safe operations
}
```
```rust
// The compiler throws an error for omission of the unsafe keyword.
fn foo() {
    // safe operations
    unsafe {
        // unsafe operations
    }
    // safe operations
}
```
```rust
// This function contains unsafe blocks, but is marked with #[safe_interface],
// and therefore the compiler permits the omission of the unsafe keyword.
// It will be treated like any other safe Rust function.
#[safe_interface]
fn foo() {
    // safe operations
    unsafe {
        // unsafe operations
    }
    // safe operations
}
```
A snippet from Vec, with the proposed changes (including unsafe fields):
```rust
// Invariants:
//     'ptr' points to memory that can hold at least 'cap' instances of T.
//     'ptr' points to precisely 'len' valid (initialized) instances of T.
//     'len' <= 'cap'
// These invariants cannot be broken by safe code, since the fields can
// only be modified from within unsafe blocks.
struct Vec<T> {
    unsafe len: uint,
    unsafe cap: uint,
    unsafe ptr: *mut T,
}

// We assert that the use of unsafe code here is actually safe, and upholds
// the invariants we have imposed upon the struct. Notice that initialization
// of Vec objects must occur within unsafe blocks.
#[safe_interface]
pub fn with_capacity(capacity: uint) -> Vec<T> {
    if mem::size_of::<T>() == 0 {
        unsafe {
            Vec { len: 0, cap: uint::MAX, ptr: &PTR_MARKER as *const _ as *mut T }
        }
    } else if capacity == 0 {
        Vec::new()
    } else {
        let size = capacity.checked_mul(&mem::size_of::<T>())
                           .expect("capacity overflow");
        unsafe {
            let ptr = allocate(size, mem::min_align_of::<T>());
            Vec { len: 0, cap: capacity, ptr: ptr as *mut T }
        }
    }
}

// The unsafety is declared in the interface since we cannot guarantee the
// invariants will be upheld here.
pub unsafe fn from_raw_parts(length: uint, capacity: uint,
                                 ptr: *mut T) -> Vec<T> {
    unsafe {
        Vec { len: length, cap: capacity, ptr: ptr }
    }
}

```

# Drawbacks

The proposed solution requires that an extra attribute be added to the language, as well as extending the unsafe keyword to fields. Such efforts might be considered a waste of time or unnecessary extra complexity for the language / compiler.

# Alternatives

We can keep going with the current system. However, incorrectly-written unsafe code may come back to bite us in the future by leading to stability issues and security vulnerabilities in Rust programs.

As an alternative to `#[safe_interface]`, another name like `#[override(safe)]` could be used.

# Unresolved questions

Is it debatable whether or not there is value in having unsafe fields be readable by safe code. It may be possible to contrive situations where it is certainly not a good idea. Perhaps unsafe fields should be completely hidden from safe code.
