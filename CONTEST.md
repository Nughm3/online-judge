# Contest format specification

The contest structures are deserialized with `serde` and are defined in [`contest.rs`](/src/contest.rs). The environment variable `CONTEST_DIR` defines a directory potentially containing multiple contests, which will all be loaded on startup.

Here is an example contest for the first 3 problems of the [CSES Problem Set](https://cses.fi/problemset):

## Directory structure

There is a `contest.md` in the top-level directory, and multiple sub-directories for each task, containing a `task.md` and a `tests` folder which contains tests in the form of `*.in` and `*.out` files.

```
CSES-Problem-Set
├── contest.md
├── weird-algorithm
│  ├── task.md
│  └── tests
│     ├── 1.in
│     ├── 1.out
│     ├── 2.in
│     ├── 2.out
│     ├── 3.in
│     ├── 3.out
│     ├── ...
│     ├── 14.in
│     └── 14.out
├── missing-number
│  ├── task.md
│  └── tests
│     ├── 1.in
│     ├── 1.out
│     ├── 2.in
│     ├── 2.out
│     ├── 3.in
│     ├── 3.out
│     ├── ...
│     ├── 14.in
│     └── 14.out
├── repetitions
│  ├── task.md
│  └── tests
│     ├── 1.in
│     ├── 1.out
│     ├── 2.in
│     ├── 2.out
│     ├── 3.in
│     ├── 3.out
│     ├── ...
│     ├── 12.in
│     └── 12.out
```

## Markdown files

Markdown files support [GitHub Flavored Markdown](https://github.github.com/gfm/) and YAML frontmatter.

`contest.md` at the contest root is used for the contest home page and the frontmatter defines configuration values for the entire contest.

`task.md` in each task directory is used for the task page and defines configuration values for the task.

## Configuration

See [`contest/loader.rs`](/src/contest/loader.rs) to see how contests are configured. I have deliberately avoided documenting it in this document as it might become outdated quickly.
