- Feature Name: read_to_string_without_buffer
- Start Date: 2015-03-13
- RFC PR: https://github.com/rust-lang/rfcs/pull/970
- Rust Issue:

# Summary

Add back `read_to_string` and `read_to_end` methods to the `Read` trait that
don't take a buffer.

# Motivation

While the `fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<()>` and
`fn read_to_string(&mut self, buf: &mut String) -> Result<()>` APIs are more
efficient removing the APIs that don't require passing a buffer entirely comes
at with convenience loss in some situations. In particular if one want's to
implement a chaining API and doesn't care about efficiency.

Today we either have to write this

```rust
fn get_last_commit () -> String {

    let output = Command::new("git")
                    .arg("rev-parse")
                    .arg("HEAD")
                    .output()
                    .ok().expect("error invoking git rev-parse");

    let encoded = String::from_utf8(output.stdout).ok().expect("error parsing output of git rev-parse");

    encoded
}
```

Or this:


```rust
fn get_last_commit () -> String {

    Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .map(|output| {
                String::from_utf8(output.stdout).ok().expect("error reading into string")
            })
            .ok().expect("error invoking git rev-parse")
}
```

But we'd like to be able to just write

```rust
fn get_last_commit () -> String {

    Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .spawn()
            .ok().expect("error spawning process")
            .stdout.read_to_string()
            .ok().expect("error reading output")
}
```

This was possible before but since there is not such `read_to_string` API
anymore, it's currently impossible.


# Detailed design

Add back methods with following signature

`fn read_to_end(&mut self) -> Result<Vec<u8>>`

`fn read_to_string(&mut self) -> Result<String>`

# Drawbacks

Two more methods to maintain

# Alternatives

Don't do it and force users to use things like `map` for chaining

# Unresolved questions

None.
