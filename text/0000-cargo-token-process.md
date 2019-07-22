- Feature Name: `cargo_token_process`
- Start Date: 2019-07-22
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a cargo setting to fetch registry authentication tokens by calling an
external process.

# Motivation
[motivation]: #motivation

Some interactions with a registry require an authentication token, and Cargo
currently stores such token in plaintext in the [`.cargo/credentials`][creds]
file. While Cargo properly sets permissions on that file to only allow the
current user to read it, that's not enough to prevent other processes ran by
the same user from reading the token.

This RFC aims to provide a way to configure Cargo to instead fetch the token
from any secrets storage system, for example a password manager or the system
keyring.

[creds]: https://doc.rust-lang.org/stable/cargo/reference/config.html#credentials

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Suppose a user has their authentication token stored in a password manager, and
the password manager provides a command, `creds cargo`, to decrypt and print
that token in a secure way. Instead of also storing the token in plaintext, the
user can add this snippet to their own `.cargo/credentials` to authenticate
with crates.io:

```toml
[registry]
token-process = "creds cargo"
```

When authentication is required Cargo will execute the command and use its
output as the token, which will never be stored by Cargo on disk. The command
will be executed inside the system's shell environment, to allow the usage of
CLI utilities:

```toml
[registry]
token-process = "creds cargo | awk '{print($2)}'"
```

It will be possible to use `token-process` on both crates.io and alternative
registries.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new key, `token-process`, will be added to the `[registry]` and
`[registries.NAME]` sections of the `.cargo/credentials` configuration file.
When a `token` key is also present, the latter will take precedence over
`token-process` to maintain backward compatibility, and a warning will be
issued to let the user know about that.

When a `cargo` subcommand needs the authentication token, Cargo will execute
the string contained in the configuration key with the system shell (`cmd.exe`
on Windows and `sh` on other platforms). If the command returns the `0` exit
code, the stardard output (with trimmed newlines) will be treated as the
authentication token. Otherwise an error message will be shown to the user,
along with the standard output.

The following environment variables will be provided to the executed command:

* `CARGO` - Path to the `cargo` binary executing the command.
* `CARGO_REGISTRY_NAME` - Name of the registry the authentication token is for.

# Drawbacks
[drawbacks]: #drawbacks

This RFC requires cargo to execute a command with the system shell, which could
make it more difficult to port Cargo to a new operative system without a shell.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The solution proposed by this RFC isn't tied to any secret storage services and
can be adapted to work with virtually any secret storage the user might rely
on, while being relatively easy to understand and use.

An alternative with better user experience but more limited customization would
be for Cargo to provide cross platform, native integration with the most popular
secret storages, for example the system keyring:

```toml
[registry]
system-keyring = true
```

The issue with the native integration proposal is it helps only a subset of
users, and it requires Cargo to implement and test integrations with each
secret storage we expect a lot of users to use.

# Prior art
[prior-art]: #prior-art

Multiple command line tools implement this system or a similar one to retrieve
authentication tokens or other secrets:

* [awscli][awscli] includes the `credentials_process` setting with the same
  behavior as the one proposed in this RFC.
* [Docker CLI][docker] offers "credential stores", programs the Docker CLI
  calls with specific arguments expecting JSON output. Implementations are
  provided for common storage systems, and the protocol is documented for users
  who want to integrate with their custom system.
* [Ansible Vault][ansible] allows to specify an executable file as the
  decryption password, executing it when needed.

[awscli]: https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sourcing-external.html
[docker]: https://docs.docker.com/engine/reference/commandline/login/#credentials-store
[ansible]: https://docs.ansible.com/ansible/latest/user_guide/vault.html#providing-vault-passwords

# Unresolved questions
[unresolved-questions]: #unresolved-questions

*Nothing here yet.*

# Future possibilities
[future-possibilities]: #future-possibilities

To allow for a better user experience for users of popular secret storages the
community could create Cargo plugins that easily integrate with such systems.
For example, an hypothetical Cargo plugin to integrate with the system keyring
could allow users to add this configuration snippet:

```toml
[registry]
token-process = "cargo credentials-system-keyring"
```

Encrypting the stored tokens or alternate authentication methods are out of the
scope of this RFC, but could be proposed in the future to provide additional
security for our users.
