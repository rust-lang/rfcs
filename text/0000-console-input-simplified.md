- Feature Name: `console_input_simplified`
- Start Date: 2021-10-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3183)

# Summary

To offer macros like `input!(TYPE);` , which would read a value of this type from stdin and return the value. Another macro that would be useful is `inputln!();`, which would read an entire line from stdin and return a String. For example, to read an *i32* you would write: `input!(i32);`, and it would essentially return an Result\<i32><i32>.

# Motivation

This would signifigantly simplify command line applications, and not require developers to use an entire crate just for simpler console input. As of right now, the proper way to read input from stdin (as far as I am aware) is something like this:

```rust
let mut input = String::new();

match std::io::stdin().read_line(&mut input) {
    Ok(_) => {}
    Err(_) => return Err(()),
}

match input.trim().parse::<TYPE_WE_WANT>() {
    Ok(x) => Ok(x), // we have the value
    Err(_) => Err(()), // failed to get the value
}
```

While this works, it's quite inconvenient. Imagine if you had to do this much work for simple console output instead of `println!()`? I feel like since we already have a convenient feature for console output, we should have a convenient feature for console input too.

# Guide-level explanation

There would be two macros, offering two different features for console input.

The first macro, `input!()` would allow you to read a single type from input. It would return a `Result` containing a value of the type specified, or nothing if it failed to read/parse that specific value type from the input provided in the console.

An usage example could look like:

```rust
let name = input!(String).expect("Invalid input!");
let age = input!(i32).expect("Invalid age!");

println!("Hello {}! I see you are {} years old!", name, age);
```

Another macro that could be provided would be `inputln!()`. This macro would return an entire line of input (up to a \n from stdin) of text, as a `Result<String>`. A usage example could be:

```rust
let message = inputln!().expect("Failed to read input!");

println!("Your message was: {}", message);
```

# Reference-level explanation

I have written a simple example implementation for these macros, and I would greatly appreciate any kind of feedback.

```rust
// NOTE THIS DOES NOT WORK IN RUST PLAYGROUND
// SINCE IT USES STDIN, SO TRY IT ON YOUR MACHINE

// Implementation

fn input<T: std::str::FromStr>() -> Result<T, ()> {
    let mut input = String::new();

    match std::io::stdin().read_line(&mut input) {
        Ok(_) => {}
        Err(_) => return Err(()),
    }

    match input.trim().parse::<T>() {
        Ok(x) => Ok(x),
        Err(_) => Err(()),
    }
}

macro_rules! input {
    ($name:ty) => {
        input::<$name>();
    };
}

macro_rules! inputln {
    () => {
        input::<String>();
    };
}

// Usage Example

fn main() {
    let name = inputln!().expect("Failed to read name!");
    let age = input!(i32).expect("Invalid age!");
    
    println!("Hello {}! I see you are {} years old!", name, age);
}
```

For my implementation, I am simply wrapping over the std::io::stdin() function calls and simplifying this into a single function, and then providing simple macros to simplify that even further.

One thing I would probably do differently in this implementation is, for the `input!(TYPE);` macro, make it so it only reads from `stdin` until it hits a ` ` (space), similar to how C++'s `std::cin` works. That way if someone typed in `10 10` into their console, we could read both by calling `input!(i32);` twice.

# Drawbacks

Honestly, I see no reason not to implement this. To implement this is the same reason why `println!()` was probably implemented, so simplify console input/output.

I feel like not implementing this only makes working with console input more of a hassle, and forces developers to either redundantly write their own functions to simplify it (reinventing the wheel over and over again), or forced them to use an entire crate for this simple feature. Forcing them to do this would be like forcing them to use an entire crate just for `println!()`.

# Prior art

This feature does exist in other languages such as Python, with it's `input()` function. It is a lot easier to read input with this function, however Python's `input` function also takes in a message that it outputs when reading input, which is not really necessary here as I think being more explicit and adding a `print` or `println` call before your `input!` call is better.

# Future possibilities

This feature would simply simplify any command line driven applications, and would even be nicer for learning purposes. A lot of times, the first programs you write when learning a new language or just programming in general is simple command line applications, that usually involve printing and getting input from the user at the console level. This would greatly improve that experience.

Another future possibility is to implement this kind of feature for Streams, such as file streams. We could eventually implement something like, for example, `read!()` which could read a value from a file. An example of this could be `read!(f, i32);`, where `f` would be the variable that held the stream handle.
