# Online Judge

Online judge for competitive programming contests.

## Installation

```bash
git clone https://github.com/Nughm3/online-judge
cd online-judge

cargo install --path .
```

Use the `online-judge` command to start the server.

## Configuration

Basic configuration is done through environment variables. `.env` files are supported.

| Environment Variable | Description                                   | Default             |
| -------------------- | --------------------------------------------- | ------------------- |
| `SERVER_ADDRESS`     | Address to listen on                          | `0.0.0.0:80`        |
| `DATABASE_URL`       | Location of the SQLite database               | `sqlite://judge.db` |
| `CONTEST_DIR`        | Location of the contests                      | `contests`          |
| `STATIC_DIR`         | Location of the [`static`](/static) directory | `static`            |
| `JUDGE_CONFIG`       | Location of the judge config file             | `judge.toml`        |

See [`main.rs`](/src/main.rs) for more information on how these environment variables are loaded.

Since the online judge is a Rust program, it also uses some conventional environment variables for logging and backtraces:

| Environment Variable | Description                                                  | Default             |
| -------------------- | ------------------------------------------------------------ | ------------------- |
| `RUST_LOG`           | Log level to use (`trace`, `debug`, `info`, `warn`, `error`) | unset (none)        |
| `RUST_BACKTRACE`     | Whether or not to enable backtraces (set to `1` to enable)   | unset               |

## Contest format

Contests are stored in an on-disk format, loaded on startup. The contest format is specified in more detail in [CONTEST.md](/CONTEST.md).

## Security

The judge executor uses a sandbox that uses Linux security APIs to control the execution of code submissions. These APIs include:

- [seccomp](https://man7.org/linux/man-pages/man2/seccomp.2.html)
- [landlock](https://landlock.io/)
- [rlimit](https://man7.org/linux/man-pages/man2/setrlimit.2.html)

Due to the current lack of security auditing, it is recommended to sandbox the **entire judge process** for security reasons. This could be done by running it in a container or VM.

## License

Licensed under the [MIT License](/LICENSE).
