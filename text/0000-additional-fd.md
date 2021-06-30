- Feature Name: additional-fd-command
- Start Date: 2020-05-31
- RFC PR: https://github.com/rust-lang/rfcs/pull/2939

# Summary

While spawning a child process, sometimes additional pipes rather than stdin, stdout, stderr are used for IPC. So std::process::Command::new should support it like the nodejs 
child_process.spawn which has the option to do it.

# Motivation
* Applications like chrome uses fd 3 and 4 for remote debugging. Tools like puppeteer use the nodejs child_process.spawn function which can do so. 
* Pipes are often used for processes to communicate with each other. These processes often communicate using pipes other than stdin, stdout, stderr like the above example.

# Design
Here is an example for how it can be

Parent process:
```rust
let (read3, write3) = std::io::pipe();
let (read4, write4) = std::io::pipe();

let process = Command::new("prog")
              .stdin(Stdio::null())
              .stdout(Stdio::null())
              .stderr(Stdio::null())
              .pass(&[&read3, &write4])
              .spawn().unwrap();
write3.write("Foo").unwrap();

let mut s = String::new();
read4.read_to_string(&mut s).unwrap();
```

The pipe() function can return a tuple of PipeReader and PipeWriter

The pass method in std::process::Command can take a slice of &dyn RawFile where RawFile is a trait that specifies
* The raw fd/HANDLE
* The flags associated with it

Child process:
```rust
let read3 = std::io::PipeReader::from_fd(3);
let write4 = std::io::PipeWriter::from_fd(4);

let mut s = String::new();
read3.read_to_string(&mut s).unwrap();

write4.write("Bar").unwrap();
```

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

The child process can get the HANDLE from fd even _without_ needing the CRT using GetStartupInfo

# Alternative

Write code using the C API for windows and unix instead of std::process:Command

# Prior art
NodeJS has child_process.spawn which has options.stdio so that many other pipes can be created and it is frequently used for interprocess communication between node js applications.