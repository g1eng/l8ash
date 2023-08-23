# About

__*leash*__ is a command line shell which is designed to have the least attack surface 
on its command line interface.

If you need to **leash shells and shell users**, you would like to permit certain operation only on your shells
with pre-defined operational procedures, which contain a set of commands and corresponding arguments, without
any unnecessary statements including  shell variables, variable expansions, inline command invocation,
declaration of shell functions nor command expansions.

`leash` provides very limited shell features but strong support to restrict command invocation.
You can permit users only to do typical administration tasks with pre-defined pipelines and its environments.
To enable this feature, a runtime configuration file (~/.leashrc) is required and its whitelist table must contain `named pipelines` (pipeline alias) for target operations.
Optionally, leash can also check the integrity of command binaries when it is invoked on the shell (as a pre-defined pipeline).

`leash` empowers you to protect systems and assets you should keep it always safe.

# Installation

```shell
make
sudo make install
```

If you do not have permission to access to the system path, you can install it under your home directory:

```shell
make
PREFIX=$HOME/local make install

# set your PATH if you need
# echo PATH=\$HOME/local/bin:\$PATH | tee -a $HOME/.bashrc
```

# Quick start

### 1. Simply invoke it as a program

```shell
$ leash
```

### 2. Feed an acceptable shell script

```shell
$ cat some_leash_script.sh
#!/bin/sh
ls -l | tr -d \\\n 
$  cat some_leash_script.sh | leash
```

### 3. Feed shell script as the argument

```shell
$  leash some_leash_script.sh 
```

### 4. Set *leash* as the user's default shell

```shell
{ 
  [ -x /bin/leash ] || {
    echo leash not found >&2
    false
  } &&
  grep /bin/leash /etc/shells > /dev/null || {
    echo failed to set leash as your default shell. consider to add `/bin/leash` to your /etc/shells. >&2
    false
  } 
} && chsh -s /bin/leash 
```

### 5. Play.

```shell
ls -l | awk {gsub("-","neko",$0);print;} | tr 0  @ | tee -a something.funny | bzip2 | dd of=sf.bz2
```

# Features

* Generic commandline interface to invoke commands with raw argument, **without any shell variables and shell functions**.
* Some of POSIX shell functionalities are **NOT IMPLEMENTED** to achieve the hardened shell experience. The leash has:
  - No builtin commands (no `echo`, `printf`, `cd`, `kill` nor `exit` as a builtin. No other builtins in the world too.)
  - No shell variable `var=val` and `$var`
  - No expansion (no path expansion with * or other special glob characters, neither variable nor command expansion.)
  - No command alias `alias name="cmd arg1 arg2"`
  - No shell function `function f1 { ... }`, `f1(){ ... }` nor `function f1 () { ... }`
  - No string literal `'...'` nor `"..."`
  - No subcommand `(...)`
  - No group command `{...}`
  - No background tasks `cmd &`
  - No redirection `cmd > file` nor `cmd >> file`
  - No indirection `cmd < file`
  - No command termination with semicolon `cmd1; cmd2; cmd3`
* **No string literals** (said again). So you should write the string `"a b c "` as `a\ b\ c\ `.


* **Pipeline**: Ordinal pipeline for system shell. It is only the way to modify temporary input/output in a shell session.
* **Runtime configuration**: You can write operation **whitelist** and other configuration in ~/.leashrc.
* Command **whitelist**: leash prohibits any commands other than listed names (named pipeline) on the whitelist table.
* **named pipeline / pipeline alias**: permitted operations can be declared as **named pipeline**s in a configuration file.
* **Environmental variables**: Environmental variable for a pipeline can be specified and applied to all command in the pipeline.
* **Integrity checker**: leash can check the integrity of command binaries which composes a pipeline.

# Configuration Tips

## Make whitelist only to permit specific programs

To run leash in restricted mode, create `~/.leashrc` and declare `[[white_list]]` in that:

```
[[white_list]]
name = "ls"
command_line = "/bin/ls"
env = []
integrity = []
```

With this configuration, user on the leash session cannot execute program, other than `/bin/ls`.
For an operation with a single program like this case, `command_line` fields should be a full path of the program and its arguments.

## Set pipeline aliases (or named pipeline) on the whitelist

You can declare command alias in a whitelist table.
For the previous example, set an alias in the `name` field for the operation:

```
[[white_list]]
name = "l"
command_line = "/bin/ls"
env = []
integrity = []
```

With this configuration, you can invoke `/bin/ls` with the name (alias) `l`, but not with its real name `ls`.
You cannot invoke `/bin/ls` with its full path or its real name. (and you cannot invoke the program named `l`, if it exists in your PATH).

Also, you can declare pipeline alias with the same mechanism. Set pipeline statement in the command attribute like following:

```
[[white_list]]
name = "count_files"
command_line = "/bin/ls | wc -l"
env = []
integrity = []
```

## Pass environmental variables for a pipeline

```
[[white_list]]
name = "kci"
command_line = "/home/mofuzawa/bin/kubectl cluster-info"
env = ["KUBECONFIG=/var/conf/dist/your-kube-config"]
integrity = []
```

## Check integrity for each command for a pipeline

```
[[whitelist]]
name = "lstr"
command_line = "/bin/ls -l | /bin/tr - o"
env = []
integrity = [
        "a3604f3968fda1471dfdb51a3a4454d8a1b6c3dead99e84f442b515b9b49da53",
        "3138ff15c875f111613407f39261babafbfe8cdc77a4c1cebb834334b78b9f0b",
]
```

#### ATTENTION

For integrity checking, all command must be spelled with its full path, unless the command is not invokable because of failure on path discovery.

# Design concept

See the second clause of the [Features](#Features) above. 
Each condition, which means the lack of the generic shell feature, is a building block of the <u>**leash security model**</u>.

| Specification                  | Description (especially for the security)                                                            |
|--------------------------------|------------------------------------------------------------------------------------------------------|
| No builtins                    | No hack with shell builtins                                                                          |
| No shell variable              | No worry about any dangerous contents inside variables                                               |
| No expansion                   | No worry about unexpected expansion to be evaluated as malformed commands or strings                 |
| No command aliases             | No new set of attack codes in the shell session                                                      |
| No shell functions             | No new set of attack codes in the shell session                                                      |
| No string literal              | No confusion of several equivalent text expressions. Escape character is the only permitted way.     |
| No subcommand                  | No fork in the shell by itself. Only processes are spawned by the shell in a pipeline                |
| No group command               | No bundle of stdout/stderr. A command has single I/O in a pipeline.                                  |
| No background tasks            | No unmanaged processes which is hanged up after the spawning.                                        |
| No redirection nor indirection | No read/write operation for the shell itself. Filesystem I/O is only permitted for commands.         |
| No semicolon                   | EOL is the only-one op code for the list evaluation. Thus, a list must be a pipeline in the *leash*. |

In addition, leash ensures users only to invoke trusted programs via whitelist.

### ATTENSION

Leash does not cover the protection of filesystem or its contents.
It is recommended to use another mechanism to protect filesystem from potentially malformed programs or exploits.
The risk of overwriting leashrc or leash binary itself, is a critical factor for the leash safety.

# Bug reports

Make issues, thanks!
