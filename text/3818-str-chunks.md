- Feature Name: `str-chunks`
- Start Date: 2025-05-24
- RFC PR: [rust-lang/rfcs#3818](https://github.com/rust-lang/rfcs/pull/3818)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This proposal introduces a new method - `chunks` - for string slices (`&str`).
The std currently provides various methods for chunking slices (`array_chunks`) and iterators (`chunks`, `chunks_exact`, `rchunks`, `array_chunks`, `utf8_chunks`, ...).
However, there is no equivalent method for string slices. This proposal aims to fill this gap by introducing a `chunks` method for `&str`, which will return an iterator over substrings of the original string slice.


# Motivation
[motivation]: #motivation

Chunking is an action that may often be needed when working with data that can be seen as an iterator.
This is why there are methods for this with slices and iterators.
But, there are none for &str even tho it can be useful a lot of time!
Here are some examples:

- Converting binary or hexadecimal strings into an iterator of an integer.
Currently we would do
```rs
let hex = "0xABCDEF";
let values = hex[2..]
    .bytes()
    .array_chunks::<2>()  // unstable
    .map(|arr| u8::from_str_radix(str::from_utf8(&arr).unwrap(), 16))  // .unwrap()

// Instead of possibly doing

let values = hex[2..]
    .chunks(2)
    .map(|str| u8::from_str_radix(str, 16))
```

- Processsing some padded data like `hello---only----8-------chars---`
- Wrapping some text safely
```rs
let user_text = "...";
user_text.chunks(width).intersperse("\n").collect::<String>()
```

Overall, everything that is about handling data with repetitive pattern or with some wrapping or formatting would benefit from this function.

Another problem is that, `array_chunks` doesn't have the same behaviour as `slice::chunk` since the last element is discarded if it doesn't do the same size as `chunk_size` which isn't always wanted.
But, if you want to achieve the same thing in the current context, you will have create an unecessary vector:
```rs 
let vec = "hello world".chars().collect::<Vec<_>>(); // Really inneficient
vec.as_slice().chunks(4) // ["hell", "o wo", "rld"]
// instead of just
"hello world".chunks(4) // ["hell", "o wo", "rld"]
```
It's
1. more code
2. less readable
3. owning some unecessary data
4. losing the borrowing lifetime of the initial string slice
```rs
fn example_when_owning(s: &str) -> Vec<&str> {
    let vec = "hello world".bytes().collect::<Vec<_>>();
    vec.as_slice()
        .chunks(4)
        .map(|bytes| str::from_utf8(bytes).unwrap())
        .collect() // Error! The function tries to return some borrowed data (str::from_utf8) declared in this function
}

fn example_when_borrowing(s: &str) -> Vec<&str> {
    "hello world".chunks(4).collect() // works fine!
}
```

Also, `str::chunks()` is faster than `Chars::array_chunks()` (without even considering `str::from_utf8().unwrap()`)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`slice::chunks` but for `&str` and with chunks being substrings as `&str`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

- Create a new `str::Chunks` in `core/src/str/iter.rs` and implement `Iterator` & `DoubleEndedIterator` on it
- Create a new method on `str`:
```rs
pub fn chunks(&self, chunk_size: usize) -> str::Chunks<'_> {
    str::Chunks::new(self, chunk_size)
}
```

_Implementation at <https://github.com/tkr-sh/rust/tree/str-chunks>_

# Drawbacks
[drawbacks]: #drawbacks

`.chunks()` on `&str` isn't necessary clear if it's on `u8` or `char`. Tho, if items in the chunks are `&str` it makes sens that it's on `char`s.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- `.chars().collect()` then `vec.as_slice().chunks()` but it's significantly longer and is owning data that could be avoided. See [motivation](#motivation).
- `.chars().array_chunks()` but it's unstable, slower and doesn't behave in the same way. See [motivation](#motivation).

# Prior art
[prior-art]: #prior-art

- `slice::chunks(usize)`
- `str::chars()`
- `Iterator::array_chunks(usize)`

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

None.
