- Feature Name: `social_unsafe`
- Start Date: 2019-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow experienced authors to use `unsafe` without getting in the way.

# Motivation
[motivation]: #motivation

Our society is based on trust systems. We generally only trust people to do dangerous things if they are trustworthy.

Trust can be built up by a history of trustworthy acts. If a person does good things™, trust in them increases.

Rust has pioneered the world of safe programming by separating safe from unsafe programming. But do we really want to allow untrustworthy people to yield the powers of `unsafe`?

We propose that as the individual programmer grows in experience, the compiler allows them to use more and more dangerous features. This is implemented using a "Rust Credit Score" system where each programmer can improve their points over time and unlock more and more unsafe and dangerous features.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`social_unsafe` helps you as developer grow inside the Rust language as well as the ecosystem.
With every merged Pull Request and download of your published crates you gain points.
After a while you unlock new achievements.

Each level enables one additional feature:

1. Casting references to raw pointers
2. Opening `unsafe` blocks and calling unsafe functions eg. for external C functions
3. Defining `unsafe fn` for more interoperability
4. Using `union` and field access
5. Cast float to integer numbers

## Examples

### 1. Casting references to raw pointers

The more safe version is this
```rust
let p = some_list.as_ptr();
```

But with your advanced knowledge you are now allowed to use this
```rust
let p = &my_object as *const MyObject as usize;
```

With this tool you will gain a deeper understanding of how the operating system is allocating memory.

### 2. Opening `unsafe` blocks and calling unsafe functions eg. for external C functions


```rust
#[link(name = "the_great_dictator")]
extern "C" {
    fn foreign_function_who_am_i(n: usize) -> *const c_char;
}

unsafe {
    let name = CStr::from_ptr( foreign_function_who_am_i(42) );
    // prints "I can rule the world and my name is Aladin!"
    println!("I can rule the world and my name is {}!", name);
}
```

### 3. Defining `unsafe fn` for more interoperability

You are now allowed to define your very own `unsafe fn` :tada:

```rust
unsafe fn what_is_my_name() -> CString {
    CString::from_raw( foreign_function_who_am_i(42) )
}
```

As a bonus your function can be marked as `extern` so it becomes accessible from eg. C.
```rust
#[no_mangle]
pub extern "C" fn callable_from_c(x: i32) -> bool {
    x % 23 == 0
}
```

### 4. Using `union` and field access

You have reached the holy grail of dark magic memory:

Creating data structures that overlay in memory!

```rust
union U {
    i: i32,
    f: f32,
}

fn main() {
    let u = U { i: 42 };
    unsafe {
        println!("i: {:?}\nf: {:?}", u.i, u.f);
    }
}
```

Where before the compiler knew what variant you were currently using and made sure nothing broke:
```rust
enum E {
    I(i32),
    F(f32),
}

fn main() {
    let i = E::I(42);
    let f = E::F(42.0);
    println!("i: {:?}\n", i, f);
}
```

### 5. Cast float to integer numbers

Now you are allowed to cast floats with a numerical range exceeding the range of integer values.
This can lead to very subtle and hard to detect bugs, use with care!

Eg. casting an f32 value of `-21195443000000000000000000000000000000.0` to an i32 will result in the value `-2147483648`.

```rust
unsafe fn resurrect_union(f: f32) -> U {
    U { i: f as i32 }
}
```

This also allows you to build a safe abstraction
```rust
fn checked_cast(f: f32) -> Option<i32> {
    let f = f.round();
    if f.is_finite() {
        if f >= std::i32::MIN as f32 && f <= std::i32::MAX as f32 {
            return Some(f as i32);
        }
    }
    None
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Technically we will use one of the most power efficient blockchains today: pijul.org - the successor of git.

Every day during the witching hour (24:00:00 UTC+24) we will collect the download stats and store them.
This allows us to calculate two values.

1. Over all time
2. Over the last 90 days

To give progress a chance we average the two.

# Drawbacks
[drawbacks]: #drawbacks

Some people might feel social pressure if their social score drops below the level where it is acceptable to use unsafe.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Instead of looking at the amount of crates published, updated and their respective downloads the social score might also be replaced with a clock.
So every 30 days new features become available.

Another design would root the trust of the community in physical meetings like the RustFest or RustConf conferences or similar.
Local meetups would count as well but should not weight as much.
Organizing any of the events should atleast tripple the points of the event.

Not implementing this change would loosen centalized, social control over peoples lives.
Meeting people in person would become more important again.

# Prior art
[prior-art]: #prior-art

Crates.io already holds most of the data needed for this RFC.
However, it lacks the big data and machine learning aspects to precisely measure the level and commitment of each programmer in a way the we can apply `PartialOrd` to people.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should the score be public?
Some people might try to store it in a blockchain (or any similar distributed linked list) which would negate all the positive effects this RFC has on the climate.

# Future possibilities
[future-possibilities]: #future-possibilities

One possibility would be to hire the ten developers with the highest score each year to work on Rust full-time.

To raise money for these ten developers, crates.io would be extended to become a hiring platform for Rust programmers, with a reasonable fee of eg. 1% of the salary.
