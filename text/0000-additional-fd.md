- Feature Name: additional-fd-command
- Start Date: 2020-05-31
- RFC PR: TODO

# Summary

While spawning a child process, sometimes additional pipes rather than stdin, stdout, stderr are used for IPC. So std::process::Command::new should support it like the nodejs 
child_process.spawn which has the option to do it.

# Motivation
* Applications like chrome uses fd 3 and 4 for remote debugging. Tools like puppeteer use the nodejs child_process.spawn function which can do so. 
* Pipes are often used for processes to communicate with each other. These processes often communicate using pipes other than stdin, stdout, stderr like the above example.

# Design
Here is an example for how it can be
```rust
let process = Command::new("prog")
              .stdio(&[Stdio::null(),Stdio::null(),Stdio::null(),Stdio::pipe_write(),Stdio::pipe_read()]).spawn().unwrap();
process.stdio[3].unwrap().write("Foo").unwrap();

let mut s = String::new();
process.stdio[4].read_to_string(&mut s).unwrap();
```

In std::process::Command there can be a function stdio
```rust
pub fn stdio()<T: Into<Stdio>>(&mut self,cfg:&[T]) -> &mut Command
```
In std::process::Stdio there can be pipe_read and pipe_write

There can be a field stdio in std::process::Child that takes an fd as index 

# Implementation 
In my library I had to use pipes for IPC so I did a basic rust implementation here https://github.com/Srinivasa314/alcro/blob/master/src/chrome/pipe.rs
## Unix
For unix `dup2` can be used

## Windows
Eventhough windows uses handles,the MSVC runtime does have a concept of fd's
Refer:
[open_osfhandle](https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/open-osfhandle)
[get_osfhandle](https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/get-osfhandle)


For windows `lpReserved2` of `startupinfo` in `CreateProcess` can be used
(The method used by libuv:
https://github.com/nodejs/node/blob/8ae28ff3ac49cbf83f4fc3445d63b9900f3cdcda/deps/uv/src/win/process-stdio.c#L32)

The struct should be packed and crtflags are as in https://github.com/nodejs/node/blob/8ae28ff3ac49cbf83f4fc3445d63b9900f3cdcda/deps/uv/src/win/process-stdio.c#L57

# Alternative

Write code using the C API for windows and unix instead of std::process:Command

# Prior art
NodeJS has child_process.spawn which has options.stdio so that many other pipes can be created and it is frequently used for interprocess communication between node js applications.