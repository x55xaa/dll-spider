
# 📦 dll-spider

[![License: GPL v3](https://img.shields.io/badge/License-GPL_v3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0.html)
[![Common Changelog](https://common-changelog.org/badge.svg)](https://common-changelog.org)


## Overview

Easy-to-use CLI tool to inject DLLs inside running processes.


## Installation

Build the crate in release mode with `cargo`:

```console
$ cargo build --release
```

> [!IMPORTANT]
> If you are not on Windows, you need to change the build target. To do so, first add the desired [target architecture](https://doc.rust-lang.org/nightly/rustc/platform-support.html#tier-1-with-host-tools) with `rustup`:
>
> ```console
> $ rustup target add x86_64-pc-windows-gnu
> ```
> 
> And then build the crate:
>
> ```console
> $ cargo build --target x86_64-pc-windows-gnu --release
> ```


## Usage

### Enumerate target processes

You can enumerate target processes with the following command:

```console
 $ ./dll-spider enum
```

This will return a list of process names along with their respective PIDs.


### Inject a DLL

To load a DLL inside a process run:

```console
$ dll-spider load target.dll -p 1234
```

> [!NOTE]
> 
> The target process can be identified either by its PID (with the `-p` otion) or its name (with the `-n` option).


## Documentation

- [CHANGELOG](CHANGELOG.md)
