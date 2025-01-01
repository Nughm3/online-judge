# Online Judge

Online judge for competitive programming contests.

**NOTE:** This project is an earlier version of a coding contest platform, and has now been superseded by [Nughm3/contest-platform](https://github.com/Nughm3/contest-platform).

## Installation

```bash
git clone https://github.com/Nughm3/online-judge
cd online-judge

cargo install --path .
```

Use the `online-judge` command to start the server.

## Configuration

Basic configuration is done through environment variables and command line options. Command line options take precedence. `.env` files are supported.

| Command Line Option    | Description                                   | Default             |
| ---------------------- | --------------------------------------------- | ------------------- |
| `-a`, `--address`      | Address to listen on                          | `0.0.0.0:80`        |
| `-d`, `--database-url` | Location of the SQLite database               | `sqlite://judge.db` |
| `-C`, `--contest-dir`  | Location of the contests                      | `contests`          |
| `-s`, `--static-dir`   | Location of the [`static`](/static) directory | `static`            |
| `-c`, `--config`       | Location of the judge config file             | `judge.toml`        |

| Environment Variable | Description                                   | Default             |
| -------------------- | --------------------------------------------- | ------------------- |
| `SERVER_ADDRESS`     | Address to listen on                          | `0.0.0.0:80`        |
| `DATABASE_URL`       | Location of the SQLite database               | `sqlite://judge.db` |

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
- [rlimit](https://man7.org/linux/man-pages/man2/setrlimit.2.html)

Due to the current lack of security auditing, it is recommended to sandbox the **entire judge process** for security reasons. This could be done by running it in a container or VM. A [Dockerfile](/Dockerfile) is provided.

## License

Licensed under the [MIT License](/LICENSE).
