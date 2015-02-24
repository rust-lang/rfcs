- Feature Name: my_awesome_io_error_handling
- Start Date: Tue Feb 24 22:16:31 CET 2015
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Change `Read::{read_until, read_line}` to return `io::Result<usize>`. Add a
conversion method `io::Result<usize> -> isize`.

# Motivation

The [io RFC](https://github.com/rust-lang/rfcs/blob/master/text/0517-io-os-reform.md)
removed the `EndOfFile` error kind to simplify the `Read::read` method: A read
of `Ok(0)` is now supposed to be interpreted as EOF. While this change clarifies
the meaning of `Ok(0)` and avoids having two ways to signal EOF, it complicates
certain convenience functions.

Previously `read_until` and `read_line` returned `Err(EndOfFile)` at the end of
the file. Since `read_until` and `read_line` now return `io::Result<()>`, and
since `EndOfFile` has been removed, this is no longer possible. Instead the user
has to check if the method appended any new bytes to the buffer in order to
detect EOF. For example:

```rust
let mut my_awesome_string = String::new();
let mut old_len = 0;
while buf.read_line(&mut my_awesome_string).is_ok() {
    if my_awesome_string.len() == old_len {
        break; // EOF
    }
    old_len = my_awesome_string.len();
    // handle new data
}
```

Compared to the old interface this is more flexible, but, considering that
`read_line` is a convenience method, this new design makes it less valuable.
with old_io, the code looks like this:

```rust
let mut my_awesome_string = String::new();
while let Ok(line) = buf.read_line() {
    my_awesome_string.push_str(&line);
    // handle new data
}
```

Here we propose the following changes to rectify this situation:

- Have `read_until` and `read_line` return `io::Result<usize>` where the `usize`
  is the number of bytes appended to the vector/string.
- Add a method `to_isize` to the `io::Result<usize>` type.

The `to_isize` method has the following behavior:

```rust
fn to_isize(res: io::Result<usize>) -> isize {
    match res {
        Ok(v) => v as isize,
        _ => -1,
    }
}
```

With this method, the code above can be written somewhat simpler

```rust
let mut my_awesome_string = String::new();
while buf.read_line(&mut my_awesome_string).to_isize() > 0 {
    // handle new data
}
```

Note that an `usize` in `io::Result<usize>` will never overflow `isize` in
the `Read` context because `String`, `Vec<u8>`, and `&[u8]` can never be larger
than `isize::MAX`.

# Detailed design

Change `read_until`, `read_line`, and `io::Result<usize>` as described above.
