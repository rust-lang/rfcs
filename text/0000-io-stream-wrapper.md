- Feature Name: Stream wrappers standardization
- Start Date: 2016-03-31
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Standardization stream wrappers like compression, encryption and etc.

# Motivation
[motivation]: #motivation

There are not best practices for writing libraries for compression,
encryption and etc.

Compression libraries need some method for "finish" work: write end
stream mark and flush data.

This work MUST explicit call and SHOULD NOT run in drop() method:
on reading incomplete stream much perefered unexpected end of stream
instead of false positive success.

Also "finish" of work is a good place to "unwrap" original writer
or reader.

Currenctly compression libraries have handling thing differently:

 * snappy expects the user to call flush(), and do not allow
   inner writer retrieval;
 * flate2 offers finish(), but manage the inner ownership internally
   with an Option;
 * stdlib BufWriter goes even a bit further by handling a possible
   panic during the finish();
 * lz4-rs expects the user to call finish() and unwrap inner writer.

# Detailed design
[design]: #detailed-design

For standardization stream wrappers the proposed adding traits to
standard library (`src/libstd/io/mod.rs`) traits:
```rust
pub trait ReadWrapper<R: Read>: Read {
    fn reader(&self) -> &R;

    fn finish(self) -> (R, Result<()>);
}

pub trait WriteWrapper<W: Write>: Write {
    fn writer(&self) -> &W;

    fn finish(self) -> (W, Result<()>);
}
```

This traits add to standard `Write`/`Read` traits method:

 * `reader`/`writer` for accessing wrapped stream.

   This method is useful, for example, for get statistic information
   from wrapped stream.
   Rust borrow checker guarantee safety of this method: user can't
   change wrapper object state with this method.

 * `WriteWrapper::finish` for write end of stream mark and unwrap
   original writer.

   This method used for explict write end of stream mark and unwrap
   stream. End of stream mark SHOULD NOT write in `drop` method,
   because on some write error preffer to get "unexpected end of
   stream" error instead of false positive "correct" stream end.

 * `ReadWrapper::finish` for unwrap original writer.

   This method is creater for `WriteWrapper::finish` symmetric and
   SHOULD BE used for unwrap original stream after end of stream
   reached.

   If end of stream is not reached then result of this method is
   depends by implementation.

## WriteWrapper example

Example of `WriteWrapper` using:
```rust
let mut f = try! (File::create("foo.cer"));
// Write private key
try! (f.write_all(b"-----BEGIN CERTIFICATE-----\n"));
let mut e = Base64Wrapper::new(f);
try! (e.write_all(certificate));
let mut f = try! (e.finish());
try! (f.write_all(b"-----END CERTIFICATE-----\n"));
```

# Drawbacks
[drawbacks]: #drawbacks

Support for this feature entails the loss of compatibility with many
libraries.

# Alternatives
[alternatives]: #alternatives

Keep as is.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
