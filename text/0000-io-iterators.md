- Feature Name: 
- Start Date: Fri Mar 13 20:25:02 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make `std::io` iterators more convenient by moving error handling out of the
loop.

# Motivation

`ReadExt` and `BufReadExt` provide several iterators:

* `BufReadExt::split` iterates over `Result<Vec<u8>>`
* `BufReadExt::lines` iterates over `Result<String>`
* `ReadExt::bytes` iterates over `Result<u8>`
* `ReadExt::chars` iterates over `Result<char, CharsError>`

For example, `BufReadExt::lines` can be used like this:

```rust
let mut reader = // A reader that implements `BufReadExt`.
for line in reader.lines() {
    let line: String = match line {
        Ok(l) => l,
        Err(e) => {
            // Handle the error here (or don't.)
            break;
        },
    };
    // Work with the line here
}
```

Some might consider `lines` a convenience method and would thus expect it to
simply stop the loop upon the first error. The current design, however, make the
user explicitly handle the error inside the loop.

Others want to handle every io error in their code and if `lines` just stopped
the loop upon the first error, they could not distinguish between errors and
end-of-file.

We try to find a middle ground between those two positions below.

# Detailed design

Change the signatures of the methods mentioned above to accept an error
parameter. For example for `lines`:

```rust
fn lines(self, err: Option<&mut Option<Error>>) -> Lines<Self> { /* ... */ }

impl<B: BufRead> Iterator for Lines<B> {
    type Item = String;

    // ....
}
```

The `err` argument will be stored in `Lines`. Upon the first error, lines checks
whether `err.is_some()`. If so, it stores the error in the reference. Either
way, it will stop the loop.

This function can then be used like this:

```rust
// No error handling:

let mut reader = // A reader that implements `BufReadExt`
for line in reader.lines(None) {
    // Work with the line here
}

// Error handling:

let mut reader = // A reader that implements `BufReadExt`
let mut err = None;
for line in reader.lines(Some(&mut err)) {
    // Work with the line here
}
if let Some(err) = err {
    // Handle the error here
}
```

# Drawbacks

None

# Alternatives

## What other designs have been considered?

None

## What is the impact of not doing this?

See the motivation.

# Unresolved questions

None
