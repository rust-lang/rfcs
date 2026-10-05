- Feature Name: `cargo_rustup_discovery`
- Start Date: 2022-06-04
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Tracking Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC is a proposal to fix a security issue with how Cargo and Rustup discover their files.

# Motivation
[motivation]: #motivation

Currently, both Cargo and Rustup search for their files starting in the current directory and then walk towards the root of the filesystem.
This presents a security hazard because those directories may be under the control of another user.
These files may contain instructions which can then execute arbitrary commands, giving control to the other user account.
This is particularly hazardous when running under a path that is world-writeable, such as `/tmp` on many Unix-like systems, or in some environments the root of a Windows drive like `C:\`.

This affects the following file searches:

* `Cargo.toml` (to find the "current" project and the workspace root)
* `.cargo/config.toml` [configuration files]
* `rust-toolchain` or `rust-toolchain.toml` [toolchain overrides]

This RFC proposes a new mechanism to constrain how Cargo and Rustup search for their files.
This proposal is based on the recent changes to git in response to [CVE-2022-24765] described in Appendix [Git behavior](#git-behavior).

[toolchain overrides]: https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file
[configuration files]: https://doc.rust-lang.org/cargo/reference/config.html#hierarchical-structure

## Threat examples

The primary threat this RFC is aiming to address is the situation where `cargo` or `rustc` enables other users or malicious programs to elevate their access by running arbitrary programs under your account.
This can happen with seemingly innocuous actions such as:

* Running `cargo` commands in directories that are at or below a directory that is writeable by other users.
  Some examples of this are:
  * using a typical Windows installation which has writeable root directories.
  * using `cargo install` which uses `/tmp` on some Unix-like environments.
  * trying out building a cargo project from within a directory you created in `/tmp`.
  * using a network-mounted filesystem, where other users have write access to the root of that network mount (for example, some classroom environments).
  * using a network-mounted filesystem, where the default behavior is to map all files to be owned by the user mounting it, disregarding the ownership on the individual files on the network drive.
* Running `rustc -V` in your shell prompt to show the current Rust version in the current directory (such as for Rustup overrides).

These actions require trusting every directory that these commands run in, *and* every directory above them.

We think most users will not be aware of these implicit trust requirements, and present a security hazard.

This RFC is not aiming to address the general threats of arbitrary execution of code defined in dependencies (such as build scripts or proc-macros) intentionally added by the user.
Those threats still require reviewing the code and trust relationship with those dependencies.

## Rollout

The implementation for this will initially be released as a warning to give advance notification to users about the change without suddenly breaking their environment.
A release of rustup will need to be made first, and then cargo should follow with a nightly release soon after (within a few days).

After approximately 6-12 weeks, both cargo and rustup will be changed to generate an error.
The rest of the text of this RFC discusses this proposal assuming it is generating an error.

We do not consider the severity of this threat sufficiently high relative to other threats that this change was needing to be done under embargo.
Since this risk is now publicly known, it is not expected that a warning period will be excessively risky.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When Cargo or Rustup search for their files,
they will only allow opening files that are owned by the current user.
If the file is owned by a different user,
then an error will be reported and the tool will exit with a nonzero status.

## Safe directories

To disable the ownership requirement, the "safe directories" option provides a way to specify which directories are allowed to be accessed,
or to turn off the constraint entirely.

The method for setting the safe directories option depends on whether you are using rustup and/or cargo.
When using rustup, most users should typically configure the setting via rustup described below.
If you are not using rustup, then the cargo configuration described below should be used.
The `RUSTUP_SAFE_DIRECTORIES` environment variable applies to both programs.

### Cargo safe directories config

The `safe.directories` Cargo config option is an array of strings of directories where the ownership check is not enforced.
See [Safe directories behavior](#safe-directories-behavior) for details on the checking behavior.
For example:

```toml
[safe]
directories = ['C:\Users\eric\Projects', 'D:\Other Projects']
```

Cargo will only read the safe directories option from:

* The Cargo home directory (`$CARGO_HOME/config.toml`).
* The `CARGO_SAFE_DIRECTORIES` environment variable.
* The `RUSTUP_SAFE_DIRECTORIES` environment variable.

Setting `safe.directories` in any other config location will be an error.

The `CARGO_SAFE_DIRECTORIES` environment variable supports multiple
directories separated by `:` or `;` based on the platform
(using [`std::env::split_paths`](https://doc.rust-lang.org/std/env/fn.split_paths.html)).

As with other array config values,
Cargo will append the environment variable entries to the config file entries.

The `RUSTUP_SAFE_DIRECTORIES` environment variable is also read,
and will be appended to the list.
This is to help support the scenario if you are using both Cargo and Rustup;
the config option only needs to be set in one place (with Rustup).

> Note: The `RUSTUP_SAFE_DIRECTORIES` environment variable may be inherited from the rustup proxy which may automatically set it based on rustup's global configuration.

### Rustup safe directories config

Rustup has an internal safe directories config option similar to Cargo's.
Since the settings file is private to Rustup,
the following CLI options can be used to manage this setting:

* `rustup set safe-directories add PATH`
* `rustup set safe-directories list`
* `rustup set safe-directories clear`
* `rustup set safe-directories remove PATH`

The safe directories may also be specified via the `RUSTUP_SAFE_DIRECTORIES` environment variable.
This environment variable has the same splitting behavior as Cargo described above.
These directories will be appended to the list from Rustup's config.

Rustup sets the `RUSTUP_SAFE_DIRECTORIES` environment variable when launching a tool via the [Rustup proxies].
This allows the user to configure their safe directories in one place (with Rustup),
and have tools like Cargo inherit those settings.

[Rustup proxies]: https://rust-lang.github.io/rustup/concepts/proxies.html

### Safe directories behavior

The directory listed in the safe directories list indicates the directory where it is OK to load projects owned by another user.
It will also be safe to load files from any subdirectory from the specified directory.

A special entry of `*` means to match all directories (disabling the check entirely).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Error message

It is important that the error message be as clear and helpful as possible,
since users are likely to encounter this error.
The following is an example of what the error may look like:

```text
error: `C:\Projects\MyProject\Cargo.toml` is owned by a different user
    For safety reasons, Cargo does not allow opening manifests owned by a
    different user, unless explicitly approved. Cargo may run executables as
    instructed by that file, which would grant permissions to the other user
    to run arbitrary executables under your account.

    To approve this directory and disable this error, run:

        rustup set safe-directories add "C:\Projects\MyProject"

    See https://rust-lang.github.io/rustup/safe-directories.html for more information.

    Current user: Eric
    Owner of file: Josh
```

The message for Rustup will be similar.

If Cargo is not running under rustup, then the message is adjusted to say:

```text
    To approve this directory, set the CARGO_SAFE_DIRECTORIES environment
    variable to "C:\Projects\MyProject" or edit
    `C:\Users\eric\.cargo\config.toml` and add:

        [safe]
        directories = ['C:\Projects\MyProject']

    See https://doc.rust-lang.org/cargo/reference/config.html#safedirectories for more information.
```

## `Cargo.toml` discovery

There are two situations where Cargo will search for a `Cargo.toml` file:

* The initial discovery of the first `Cargo.toml` to determine which one is the "current" one.
* The subsequent search for the workspace root `Cargo.toml`.

In these two scenarios, when Cargo attempts to load a `Cargo.toml` file,
it will check that the ownership of the file matches the current user.

The ownership check does not apply to other situations, such as:

* `path` dependencies.
* Manifests loaded from the registry or git cache.
* Manifests specified with `--manifest-path`.
* Manifests loaded from a "safe" manifest via workspace links (`package.workspace`).

## Cargo config discovery

Cargo searches from the current directory and upwards for `.cargo/config.toml` files.
When it discovers one, it will check that the ownership of the file matches the current user.

This ownership check does not apply to:

* Loading `$CARGO_HOME/config.toml`
* Loading configs from includes (currently an unstable option)
* Loading configs passed on the command-line with `--config`

## Rustup toolchain override discovery

When searching for a `rust-toolchain`/`rust-toolchain.toml` file, Rustup will check that the ownership of the file matches the current user.

If both `rust-toolchain` and `rust-toolchain.toml` exist, the ownership of both files will be checked.

## Ownership check behavior

The specifics of how ownership will be checked are:

* Unix: The user as reported by [`geteuid`] will be compared with the owner of the file.
    * If the effective UID is the root user, it will also allow files owned by the uid specified in `SUDO_UID` to make it easier to use `sudo` operations.
* Windows: The user [SID] of the current token will be compared with the SID of the file.
    * A file owned by the [Administrators Group] is also allowed to be accessed if the current user is also in the Administrators Group.

      There are several situations where files can be owned by the Administrators Group.
      This exception is added to accommodate these situations to avoid overly aggressive ownership requirements.
      For example, running a process with the "Run as Administrator" option will cause all files to be owned by the Administrators Group.
      This is the default behavior of running Powershell on GitHub Actions.

      This exception matches the behavior of git-for-windows.

The ownership check does not follow symlinks, since the owner of the symlink itself is what is important.

[`geteuid`]: https://man7.org/linux/man-pages/man2/geteuid.2.html
[SID]: https://docs.microsoft.com/en-us/windows/security/identity-protection/access-control/security-identifiers
[Administrators Group]: https://docs.microsoft.com/en-us/windows/security/identity-protection/access-control/local-accounts#administrator-account

### ACL verification

This RFC does not propose checking ownership and permissions of any ancestor directory at or above the Cargo/Rustup files, or the permissions of the files themselves.
It is assumed that it is the user's responsibility to make sure that the ownership and permissions are configured properly.

### TOCTOU

The following section addresses concerns about time-of-check to time-of-use ([TOCTOU]) races.
There are some scenarios where there may be a race, but we believe these are out of scope for this RFC.

The implementation will not implement particular measures to avoid races with verifying the ownership and loading the respective files.
Protecting against an attacker that has the ability to create files owned by the victim, or to replace an ancestor directory is out of scope for this RFC.

Races with symlinks within the current path's ancestry is out of scope for this RFC.
If an attacker has the ability to modify the symlink, they will be able to circumvent the ownership checks of this RFC.
At this time we do not consider this to be a likely configuration, and may be considered in improperly configured system.
It likely takes a considerably high level of access to be able to make such a change, which is beyond what this RFC can address.

The Linux sysctls [`fs.protected_hardlinks`] and [`fs.protected_symlinks`] provides mechanisms to prevent TOCTOU races.
For example, without the hardlink protection, an attacker can create a hardlink to a file owned by the user, though that would require the target file to have contents that could be exploited.
If these are disabled, or running on a system that does not have these protections, it may expose Cargo and Rustup to be vulnerable to a TOCTOU race involving hard or soft links.

[TOCTOU]: https://en.wikipedia.org/wiki/Time-of-check_to_time-of-use
[`fs.protected_hardlinks`]: https://www.kernel.org/doc/html/latest/admin-guide/sysctl/fs.html#protected-hardlinks
[`fs.protected_symlinks`]: https://www.kernel.org/doc/html/latest/admin-guide/sysctl/fs.html#protected-symlinks


## Implementation details

The Cargo and Rustup teams are working together to implement this change.
They will endeavor to share the validation logic as much as possible.


# Drawbacks, alternatives, and other considerations

The following sections discuss alternatives considered, drawbacks to this proposal,
and other considerations for why this approach is being considered and possible future changes.

## Comparison to git

This design is largely based on git's behavior.
However, there are several differences:

* Git restricts the safe directory comparison to only the given directory,
  and none of its subdirectories.
  This proposal distinctly deviates from that by allowing subdirectories.
  It is not clear why git did not implement that behavior.
  This was chosen because:

    * Cargo projects and configs are nested, and Cargo needs to be able to read all of them.
      Specifying individual directories could be cumbersome,
      or more complex logic like "honor a match in the root of a workspace to apply to all members" could be more difficult to implement.
    * Users may choose to trust their entire home directory (or some subset of it).
      This can be helpful if the user needs to regularly run tools as a different user across different projects.

  Unfortunately this does have a greater security exposure,
  but I think the convenience is on balance with the potential risk.

  Alternatively, gitignore-style patterns could be used (like `/home/eric/**`),
  however we would prefer to avoid the additional complexity.

* Git has the behavior that an empty entry in the safe directories list will "clear" it and ignore all previous entries.
  This is one way to override the system config.

  Cargo doesn't have system configs, so it may not be as useful.

  One possible use case is if you have `*` in your cargo home config,
  and you want to set an environment variable to override it.

* Git supports path interpolation in its paths, such as `~` expanding to the user's home directory.
  At this time, Cargo does not do interpolation for paths,
  but something like this could be considered.
  There is a hazard not supporting it in the outset that it could be technically breaking to add it in the future, but I consider that risk extremely low.

* Cargo uses `safe.directories` with a plural unlike git which uses `safe.directory`.
  Cargo typically uses plurals for arrays (such as "rustflags"),
  and I think it better conveys that it takes multiple values
  (particularly in the environment variable `CARGO_SAFE_DIRECTORIES`).
  However, this difference may cause confusion for users.

## Usability hazards

There are quite a few scenarios where this will cause a false-positive, likely causing frustration for users.
Some examples:

* Running cargo in a Docker container where the project or a config is mounted via a volume.
* Running cargo in a Docker container which calls `useradd` and then the USER instruction to switch users after fetching a Cargo project.

This change may introduce more frustration that outweighs the potential security improvements.

## `cargo config`

If Cargo detects it is running under Rustup,
then it will suggest the user run `rustup set safe-directories add PATH`
to configure the appropriate path.

However, if Cargo is not being used with Rustup,
then it currently does not have a convenient way to update the config via a CLI command.
This presents a usability drawback to this plan.
Cargo has an unstable `cargo config` command,
but it is currently unstable and does not implement the `set` command.
This could be added in the future to make it a little easier.
In the meantime, the error message will suggest setting `CARGO_SAFE_DIRECTORIES` or editing a config file.

## Cargo config search enhancements

There have been longstanding issues with how Cargo searches for config files.
This proposal makes a change that may complicate future changes to that discovery process.
Unfortunately, a plan to change the config discovery process is not on the horizon,
so it is unknown what impact this will have.

## Rustup `rust-toolchain` trust

There have been discussions about adding a mechanism to Rustup to add some kind of "trust" system for `rust-toolchain` files.
This idea would have an even stronger indication of the user being happy with a particular `rust-toolchain` file, since its contents would be hashed, and the trust would be revoked if it changes.

It is not clear to me how this explicit trust system would interact with "safe directories".
If it is added in the future, there may be some overlap or confusion on the difference between the two.

## Granular capabilities

An alternative is to associate actions and config options with granular capabilities.
Then, Cargo and Rustup could trust files that are owned by different users if they don't
present a hazard (such as executing external commands).
This is the approach being taken by [gitoxide](https://github.com/Byron/gitoxide).
It has a [`git-sec`](https://github.com/Byron/gitoxide/discussions/425) crate defining a security model.

The Git developers also considered only generating an error if the ownership differed from the current user *and* there is a `core.fsmonitor` setting in the repo.
However, they decided not to go that route.

We feel like it is unlikely we can securely add such a mechanism to Cargo and Rustup due to the complexity.
Also, in many cases, Cargo projects run code on the host
(such as proc-macros and build scripts)
which would severely limit what would be constituted as "safe".

## Ceiling directories

Git also has `GIT_CEILING_DIRECTORIES` for limiting the upwards search,
and was presented as a way for git users to protect themselves.
This can be used to avoid potentially loading malicious repositories,
but in practice it is best used for improving performance.
It might be nice for Cargo and Rustup to adopt something similar,
but it is not a replacement for ownership checks since it is opt-in.

## Honor the user of the current directory

The ownership check could include the current directory into its rules.
Some possible examples:

* Don't check the ownership of files in the current directory.
* Record the owner of the current directory, and treat that owner as "approved" and allow any files owned by that user in any parent directory.

This is under the assumption that the user running the command makes an explicit choice as to which directory they run Cargo or Rustup from.
This could significantly ease the burden of dealing with this issue,
as it will avoid needing to set safe directories in almost all use cases where they would be needed.

However, I feel like this approach has a high risk.
Some Rust users may execute commands like `rustc -V` to get the version for a shell prompt.
This means that just changing directory to another user's directory could expose them.
This was one of the leading concerns that prompted git to implement their fix, and the reasoning behind the risk seems compelling to me.

The risk for Rustup seems high due to something like the shell prompt issue,
but this exception could be considered for just Cargo which may be less likely to be exposed in a similar way.
However, that presents several drawbacks:

* If it only honors the current directory, then Cargo's behavior could be inconsistent based on which directory you are in.
  For example, if the root `Cargo.toml` is owned by a different user, it would work if you run cargo commands from that directory, but not any subdirectory.
* This behavior would be inconsistent between Cargo and Rustup.
* Setting the current working directory may not be such an obvious signal of "I trust this location".

## Filesystem behavior risks

This fix assumes that a user can't in any way create a file owned by another user
(unless they have elevated permissions like root).
I believe this is the case for all major operating systems and filesystems.
See also the [TOCTOU](#toctou) section for a caveat about protected links.

Some exceptions that present risks to this RFC:

* If a network-mounted filesystem is configured to map all files to the local user (which is not uncommon), then other users will be able to create files owned by the current user.

  In this case, the user will need to trust all users who can write to the mounted filesystem.

* UID reuse may allow an attacker to create a file that is later owned by a new, legitimate user.
  For example, an attacker is able to create a new user (or exploit something that creates a temporary user) and poison the filesystem, and then delete that user.
  Then, later, a genuine user is created with that same UID, they will unknowingly give privilege to those previously created files.

  On Windows, the SID should be reasonably unique, making this unlikely.
  For other systems, this is a exploit chain that is possible, though it requires other exploits that likely have easier ways to escalate than through Rustup and Cargo.

## Check the ownership of every directory while traversing upwards

The Git developers considered checking the ownership of every directory while traversing upwards,
but they decided not to do it for performance reasons.
I don't see a particularly strong reason to check the ownership along the way.
I'd also be concerned about the performance consequence, though I expect it is extremely small.

## Don't issue an error, and silently stop on ownership change

One option is to check the ownership while traversing upwards,
and just stop when the ownership changes without continuing upwards,
and without informing the user.

Unfortunately this option causes some issues with the way cargo and rustup work.
Cargo may behave incorrectly if it is able to read a workspace member package,
but be unable to load the workspace root.
This would cause cargo to believe the workspace member is in a workspace by itself, and start resolving it independently.
This could cause confusing errors for the user.
Similarly for cargo config files, the config file may be required for builds to work correctly, and silently avoiding them could lead to confusing errors.
Rustup toolchain overrides also have similar issues with potential confusion when using the incorrect toolchain.

## Safe directory issues with Windows, WSL, and mingw

On Windows, Git users have experienced some confusion and problems with setting safe directories.
This is because git-for-windows is a mingw application,
and needs to straddle the difference between POSIX and Windows style paths,
as well as network paths.

There is some concern that Cargo users using WSL or git-for-windows bash will have difficulty or confusion on how to set paths for the safe directories variable.
I do not think it should be too much of an issue for Cargo because Cargo never uses POSIX paths, and only uses Windows-style paths.
If the error message makes the full path clear, then the user should be able to copy-paste it.

This may be particularly difficult for a user running Windows `cargo.exe` from within WSL.
Paths in this environment look like `\\wsl$\Ubuntu-20.04\home\eric\foo`.
I don't know how common this use case is, as I would expect someone doing things in WSL would install the native rustup/cargo binaries instead of using the ones from Windows.

Another risk is entering Windows paths in a TOML config might be confusing.
Either the user needs to use a literal string, or use escapes in a normal string.
Some users have reported confusion about TOML string escaping.

## Filesystem case-sensitivity

I am proposing at this time that the safe directory check should be case-sensitive in all environments.
However, this is causing issues for git users (see <https://github.com/git-for-windows/git/issues/3801>).

This can manifest on Windows very easily.
If you have a directory `Foo`, and you type `cd foo`,
then the operating system will report the current directory is `foo`.

I am not aware of an easy way to detect if a filesystem is case-sensitive.

## Git support

Cargo does git repo discovery in several places.
At this time, I do not consider it a security concern because libgit2 does not launch executables.

libgit2 has been patched to have similar behavior as the git CLI, but Cargo has disabled the libgit2 owner validation.
Issues with refusing to load git repos in `CARGO_HOME` for example will likely be very frustrating for users.

#### Current git discovery

The following is a list of places where Cargo uses git repositories:

- Places where Cargo uses repo path discovery:
    - Package listing: Determines the list of files in the package.
        - Used in several places:
            - `cargo package`, `cargo publish`
            - `cargo vendor`
            - Checking for the "newest" file for fingerprinting (`cargo doc` and build scripts).
        - Cargo does not use git if `package.include` is set.
        - Currently Cargo ignores errors opening the repository, and falls back to a dumb walk.
            - This may be a hazard because it could silently change which files are included during publishing. Disables dirty checks and `.cargo_vcs_info.json`.
    - `cargo fix`: Checks for dirty files.
        - If Cargo fails to open the repo, it will fail requiring `--allow-no-vcs`.
    - `cargo new`:
        - Uses discovery to determine if already inside a git repository. Failure is silently ignored and assumes that it is not in a pre-existing repository.
        - Uses libgit2 to initialize a new repository.
- Places where Cargo loads directly from a specific path:
    - Loading the registry index.
        - Currently Cargo will silently try to delete the index if it fails to open, and reinitialize it. If there is a permission error, it will fail.
    - Loading a git dependency.
        - If Cargo fails to open the repo, then it will try to delete it and reinitialize and fetch it.
    - Fetching behavior:
        - Cargo may run `git gc` based on a heuristic. If `git gc` fails, Cargo will try to delete the repo and reinitialize it.
          This may present a hazard if a user is using git 2.35.2 or newer, where it may fail with the ownership check (such as running as root with an inherited `HOME`).
          The consequence is that cargo will have to refetch the git repo when it wouldn't normally.

# Appendix

## Git behavior

In response to [CVE-2022-24765], git introduced ownership checks and a
[`safe.directory`] configuration option to override them.
More about this change is described in the [GitHub blog post].

`git` searches from the current directory up to the filesystem root to find a git repo.
This is a vulnerability because the repo may contain config settings that execute arbitrary programs (particularly [`core.fsmonitor`]).
If an attacker has the ability to write to directories at or above your directory, then they can use it to escalate.

A classic example is `C:\` on Windows which in some environments is writeable by all users.

This is particularly problematic as many users place `git` commands in their shell's prompt,
causing it to open repositories from every directory the user enters.

`git` has been changed so that it will not open a repository owned by a different user, and instead return an error.
For most cases, it will check that the ownership of the directory *containing* the `.git` directory matches the current user (see also [CVE-2022-29187](#cve-2022-29187) below).

There are some workarounds:
* `GIT_CEILING_DIRECTORIES` environment variable defines paths where git will stop searching upwards. This has been in `git` for a long time.
    * Typically you would set this to something like `/home` or `C:\Users`.
    * Does not apply to the current directory or `GIT_DIR`
    * Supports a list of directories. An empty entry in the list will mean that the following entries won't be checked for symlinks (for performance).
* `safe.directory` config option (added for this CVE): A list of git repositories that are allowed to be opened, even if they are owned by someone else.
    * Can only be set in system or global config, not the repo config.
        * Support for `-c` CLI settings was implemented later, in 2.38.0.
    * Interpolated:
        * `~` is home directory
            * `~user` for a specific user
        * `%(prefix)` is git's installation directory
            * Can use literal %(prefix) by starting with `./%(prefix)`
    * The value `*` completely disables the safety check.
        Can be overridden by adding an empty entry to the list, following entries will ignore the `*`
* git-for-windows allows opening a repository owned by the `BUILTIN\Administrators` group if the current user is also a member of that group. This is necessary because the "Run as Administrator" functionality causes files created by the current user to be owned by that group.

### CVE-2022-29187

[CVE-2022-29187] was released in July 2022 which addressed an issue with the way `safe.directory` was originally implemented.
The original fix checked the ownership of the directory *containing* the git repository.
This is vulnerable in the situation where the victim owns the directory, but the attacker is able to write to it.
For example, on some Unix-like systems, if the root user is running `git` commands in the `/tmp` directory, it would trust a git repository at `/tmp/.git` (since root owns `/tmp`).
That git repository can be created by any user that can write to `/tmp`, and thus execute commands as root.

The solution is to verify the ownership of the worktree root directory (or bare repo directory), the `.git` file (for a worktree), and the `.git` directory.

#### sudo relaxation

Along with the fix for [CVE-2022-29187], additional changes were applied to relax the restrictions when running under sudo.
When running as root on non-Windows systems, it will trust directories owned by the UID indicated by the `SUDO_UID` environment variable.
This allows running `sudo` commands in a directory you own without needing to configure a safe directory.

### safe.bareRepository

Git v2.38.0 introduced the [`safe.bareRepository`] configuration setting to control the behavior around bare repositories.
This addresses a security concern when running `git` commands within a git repository that also contains a bare repository.
The concern is that users may assume the configuration of only the outer git repository is used (which is normally defined by the user), but git will use the config within the bare repository (which is under the control of the authors of the original repo).
This is a concern if you clone some external repository, and then naively navigate into a directory with a bare repo and run git commands (such as in a shell prompt).

This configuration setting supports the values "all" (the default) which allows access to any bare repository, which is the same behavior before this option was added.
The other setting is "explicit" which means it only allows access to bare repositories specified by the `--git-dir` CLI option or the `GIT_DIR` environment variable.

The current default is insecure, and it is intended to introduce a more secure option in the future.
The "explicit" setting was deemed too restrictive for most bare repository use cases.
A future enhancement may add a restriction where it only restricts bare repositories *inside* another git repository, but detecting this scenario is nontrivial.

This release also fixed the issue where `safe.directory` can now be set via the `-c` command-line option.

### Git history

* 2022-04-12 v2.35.2 — introduced original fix for [CVE-2022-24765].
  This was backported to 2.30.4, 2.31.2, 2.32.2, 2.33.3, 2.34.2.
  * <https://github.com/git/git/commit/bdc77d1d685be9c10b88abb281a42bc620548595>
  * <https://github.com/git/git/commit/8959555cee7ec045958f9b6dd62e541affb7e7d9>
* 2022-07-12 v2.37.1 — introduced the fix for [CVE-2022-29187] to handle parent directory ownership.
  This also changed how sudo support was handled.
  This was backported to 2.30.5, 2.31.4, 2.32.3, 2.33.4, 2.34.4, 2.35.4, 2.36.2.
  * <https://github.com/git/git/commit/3b0bf2704980b1ed6018622bdf5377ec22289688>
  * <https://github.com/git/git/commit/b779214eafe7fa6933524737f166ecca99bdf468>
* 2022-08-30 v2.37.3 — Windows was changed to also check for the Administrators group.
  * <https://github.com/git/git/commit/3f7207e2ea967fd2b46d9e0ae85246e93b38ed58>
* 2022-08-30 v2.37.3 — Added more diagnostic information, in particular informing Windows users about filesystems that don't support ACLs, such as FAT32.
  * <https://github.com/git/git/commit/7c83470e64eadab74689427fcd95e72f0a772ab5>
* 2022-10-03 v2.38.0 — introduced `safe.bareRepository`, and fixed the `-c` behavior.
  * <https://github.com/git/git/commit/6061601d9f1f1c95da5f9304c319218f7cc3ec75>
  * <https://github.com/git/git/commit/8d1a7448206e11cdea657c35b04cc49db39be933>
  * <https://github.com/git/git/commit/e35f202b4503256db148ad61487fe13aa75960f2>

[CVE-2022-24765]: https://nvd.nist.gov/vuln/detail/CVE-2022-24765
[CVE-2022-29187]: https://nvd.nist.gov/vuln/detail/CVE-2022-29187
[`safe.directory`]: https://git-scm.com/docs/git-config/#Documentation/git-config.txt-safedirectory
[`safe.bareRepository`]: https://git-scm.com/docs/git-config/#Documentation/git-config.txt-safebareRepository
[`core.fsmonitor`]: https://git-scm.com/docs/git-config/#Documentation/git-config.txt-corefsmonitor
[GitHub blog post]: https://github.blog/2022-04-12-git-security-vulnerability-announced/
