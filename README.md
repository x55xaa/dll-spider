
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
> ```bash
> $ rustup target add x86_64-pc-windows-gnu
> ```
> 
> And then build the crate:
>
> ```bash
> $ cargo build --target x86_64-pc-windows-gnu --release
> ```

### Release binaries

Alternatively, you can get pre-compiled binaries of every release [here](https://github.com/x55xaa/dll-spider/releases).


## Usage

### Enumerate target processes

You can enumerate target processes with the following command:

```bash
 $ dll-spider enum
```

This will return a list of process PIDs alongside their respective names.


### Inject a DLL

To load a DLL inside a process run:

```bash
$ dll-spider load target.dll -p 1234
```

> [!NOTE]
> 
> The target process can be identified either by its PID (with the `-p` option) or its name (with the `-n` option).


## Documentation

- [CHANGELOG](CHANGELOG.md)
