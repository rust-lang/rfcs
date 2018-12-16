- Feature Name: file_open_with
- Start Date: 2018-11-08
- RFC PR: 
- Rust Issue: #55762

# Summary
[summary]: #summary

This RFC proposes making `File::open()` consistent by deprecating `OpenOptions::new().read(true).write(true).open("existing_file")` and adding `File::open_with("existing_file", OpenOptions::new().read().write())` instead.

# Motivation
[motivation]: #motivation

The current way to open an existing file in read-only mode is this:

    File::open("foo.txt")

And to create a new file it is:

    File::create("foo.txt")

But if you want to open an existing file in read/write mode it is this:

use std::fs::OpenOptions;

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("foo.txt");

This is inconsistent and unexpected. I propose that we deprecate `OpenOptions::open()` and add `File::open_with(path: &str, options: &OpenOptions)`.

    let mut file = File::open_with("foo.txt", OpenOptions::new().read().write());

This matches the normal way of doing "call a method with some options" in Rust, for example `TcpStream::connect(addr)` and `TcpStream::connect_timeout(addr, timeout)`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Reading and Writing Files

To open a file in read-only mode you can use `File::open()` like this:

    let mut file = File::open("existing_file");

This will open the file in read-only mode. To create a new file and write to it you can use:

    let mut file = File::create("new_file");

This will open the file in read/write mode and create it if it doesn't already exist. If you want to open an existing file, but *not* create it if it already exists, use this code:

    let mut file = File::open_with("existing_file", OpenOptions::new().read().write());

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Internally this could be implemented exactly like this:

```
impl File {
  pub fn open_with(filename: &str, options: &OpenOptions) -> File {
    options.open(filename)
  }
}
```

# Drawbacks
[drawbacks]: #drawbacks

There would now be two ways to open a read/write file (though one would be deprecated).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is confusing, as evidenced by [these](https://stackoverflow.com/questions/50039341/open-file-in-read-write-mode-in-rust) [two](https://stackoverflow.com/questions/47956653/is-it-possible-to-use-the-same-file-for-reading-and-writing) Stackoverflow questions. Passing an "options" struct as a parameter to a `new_with()` style constructor is the standard idiom in Rust - as seen in `TcpStream::connect_timeout()` and `SipHasher::new_with_keys()`.

# Prior art
[prior-art]: #prior-art

The C API uses `fopen()` which has a `mode` string for this purpose. It doesn't have a mode structure that contains an `open()` function.

C++ has a similar mechanism - the `fstream` constructor can take `ios::in | ios::out`.

Even Haskell looks normal in comparison:

    handle <- openFile "file.txt" ReadWriteMode

I don't know of another language that has something like Rust's

    file_open("read_only")
    read_write_options.open("read_write")

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The name.

* `open_with`
* `open_with_options`
* Something else?

# Future possibilities
[future-possibilities]: #future-possibilities

This is still fairly verbose:

    let mut file = File::open_with("existing_file", OpenOptions::new().read().write());

It may be nice to add shortcuts for common options, or to use a bitfield type thing instead:

    let mut file = File::open_read_write("existing_file");
    let mut file = File::open_with_mode("existing_file", File::Read | File::Write);

Or

    let mut file = File::open_with_options("existing_file", OpenOptions::read_write());
