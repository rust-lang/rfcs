- Feature Name: `cargo_token_from_process`
- Start Date: 2019-07-22
- RFC PR: [rust-lang/rfcs#2730](https://github.com/rust-lang/rfcs/pull/2730)
- Cargo Issue: [rust-lang/cargo#8933](https://github.com/rust-lang/cargo/issues/8933)

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
the password manager provides a command, `/usr/bin/cargo-creds`, to decrypt and
print that token in a secure way. Instead of storing the token in plaintext,
the user can add this snippet to their own Cargo config to authenticate with
crates.io:

```toml
[registry]
credential-process = "/usr/bin/cargo-creds"
```

When authentication is required, Cargo will execute the command to acquire the
token, which will never be stored by Cargo on disk.

It will be possible to use `credential-process` on both crates.io and alternative
registries.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new key, `credential-process`, will be added to the `[registry]` and
`[registries.NAME]` sections of the configuration file. When a `token` key is
also present, the latter will take precedence over `credential-process` to
maintain backward compatibility, and a warning will be issued to let the user
know about that.

The `registry.credential-process` value will be used for all registries. If a
specific registry specifies the value in the `registries` table, then that
will take precedence.

The `credential-process` key accepts either a string containing the executable
and arguments or an array containing the executable name and the arguments.
This follows Cargo's convention for executables defined in config.

There are special strings in the `credential-process` that Cargo will replace
with a given value:

* `{name}` — Name of the registry.
* `{api_url}` — The API URL.
* `{action}` — The authentication action (described below).

```toml
[registry]
credential-process = 'cargo osxkeychain {action}'

[registries.my-registry]
credential-process = ['/path/to/myscript', '{name}']
```

There are two different kinds of token processes that Cargo supports. The
simple "basic" kind will only be called by Cargo when it needs a token. This
is intended for simple and easy integration with password managers, that can
often use pre-existing tooling. The more advanced "Cargo" kind supports
different actions passed as a command-line argument. This is intended for more
pleasant integration experience, at the expense of requiring a Cargo-specific
process to glue to the password manager. Cargo will determine which kind is
supported by the `credential-process` definition. If it contains the
`{action}` argument, then it uses the advanced style, otherwise it assumes it
only supports the "basic" kind.

## Basic authenticator

A basic authenticator is a process that returns a token on stdout. Newlines
will be trimmed. The process inherits the user's stdin and stderr. It should
exit 0 on success, and nonzero on error.

With this form, `cargo login` and `cargo logout` are not supported and return
an error if used.

## Cargo authenticator

The protocol between the Cargo and the process is very basic, intended to
ensure the credential process is kept as simple as possible. Cargo will
execute the process with the `{action}` argument indicating which action to
perform:

* `store` — Store the given token in secure storage.
* `get` — Get a token from storage.
* `erase` — Remove a token from storage.

The `cargo login` command will use `store` to save a token. Commands that
require authentication, like `cargo publish`, will use `get` to retrieve a
token. A new command, `cargo logout` will be added which will use the `erase`
command to remove a token.

The process inherits the user's stderr, so the process can display messages.
Some values are passed in via environment variables (see below). The expected
interactions are:

* `store` — The token is sent to the process's stdin, terminated by a newline.
  The process should store the token keyed off the registry name. If the
  process fails, it should exit with a nonzero exit status.

* `get` — The process should send the token to its stdout (trailing newline
  will be trimmed). The process inherits the user's stdin, should it need to
  receive input.

  If the process is unable to fulfill the request, it should exit with a
  nonzero exit code.

* `erase` — The process should remove the token associated with the registry
  name. If the token is not found, the process should exit with a 0 exit
  status.

## Environment

The following environment variables will be provided to the executed command:

* `CARGO` — Path to the `cargo` binary executing the command.
* `CARGO_REGISTRY_NAME` — Name of the registry the authentication token is for.
* `CARGO_REGISTRY_API_URL` — The URL of the registry API.

# Drawbacks
[drawbacks]: #drawbacks

*No known drawbacks yet.*

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The solution proposed by this RFC isn't tied to any secret storage services and
can be adapted to work with virtually any secret storage the user might rely
on, while being relatively easy to understand and use.

# Prior art
[prior-art]: #prior-art

Multiple command line tools implement this system or a similar one to retrieve
authentication tokens or other secrets:

* [awscli][awscli] includes the `credentials_process` setting which calls
  a process with arguments provided by the user. The process is expected to
  emit JSON that contains the access key.
* [Docker CLI][docker] offers "credential stores", programs the Docker CLI
  calls with specific arguments expecting JSON output. Implementations are
  provided for common storage systems, and the protocol is documented for users
  who want to integrate with their custom system.
* [Ansible Vault][ansible] allows to specify an executable file as the
  decryption password, executing it when needed.
* [Git] has a credential mechanism using store/get/erase arguments, and
  `key=value` parameters send and received with the process.

[awscli]: https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sourcing-external.html
[docker]: https://docs.docker.com/engine/reference/commandline/login/#credentials-store
[ansible]: https://docs.ansible.com/ansible/latest/user_guide/vault.html#providing-vault-passwords
[git]: https://git-scm.com/docs/gitcredentials#_custom_helpers

# Unresolved questions
[unresolved-questions]: #unresolved-questions

*No known unresolved questions yet.*

# Future possibilities
[future-possibilities]: #future-possibilities

To allow for a better user experience for users of popular secret storages,
Cargo can provide built-in support for common systems. It is proposed that a
`credential-process` with a `cargo:` prefix will use some internal support. For
example, `credential-process = 'cargo:system-keychain'`.

Additionally, the community could create Cargo plugins that implement
different storage systems. For example, a hypothetical Cargo plugin could be
specified as `credential-process = 'cargo credential-1password {action}'`.

Encrypting the stored tokens or alternate authentication methods are out of the
scope of this RFC, but could be proposed in the future to provide additional
security for our users.

Future RFCs introducing new kinds of secrets used by Cargo (i.e. 2FA codes)
could also add support for fetching those secrets from a process, in a similar
way to this RFC. Defining how that should work is outside the scope of this RFC
though.
