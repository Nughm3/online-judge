use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read, Write},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{self, ExitStatus, Stdio},
    str::FromStr,
};

pub use resource::{ResourceLimits, ResourceUsage};
use serde_with::DeserializeFromStr;
use tempfile::TempDir;
use thiserror::Error;

mod landlock;
mod resource;
mod seccomp;

#[derive(Debug)]
pub struct Sandbox {
    dir: TempDir,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, DeserializeFromStr)]
pub struct Command {
    executable: PathBuf,
    args: Vec<OsString>,
}

impl Command {
    pub fn new(
        executable: impl AsRef<Path>,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> Self {
        Command {
            executable: executable.as_ref().to_path_buf(),
            args: args.into_iter().map(|s| s.as_ref().to_owned()).collect(),
        }
    }
}

#[derive(Debug, Error)]
#[error("invalid command: {0}")]
pub struct InvalidCommand(String);

impl FromStr for Command {
    type Err = InvalidCommand;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split_whitespace();

        let Some(executable) = iter.next() else {
            return Err(InvalidCommand(s.to_owned()));
        };

        Ok(Command {
            executable: executable.into(),
            args: iter.map(|s| s.into()).collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub exit_status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub resource_usage: ResourceUsage,
}

impl Sandbox {
    pub fn new() -> io::Result<Self> {
        let dir = tempfile::tempdir()?;
        Ok(Sandbox { dir })
    }

    pub fn write(&self, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
        tracing::trace!("writing code to {}", path.as_ref().display());
        fs::write(self.path().join(path), contents)
    }

    pub fn build(&self, command: &Command, rlimits: ResourceLimits) -> io::Result<Output> {
        self.exec(command, None, rlimits, Profile::Build)
    }

    pub fn run(
        &self,
        command: &Command,
        stdin: &[u8],
        rlimits: ResourceLimits,
    ) -> io::Result<Output> {
        self.exec(command, Some(stdin), rlimits, Profile::Run)
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    #[tracing::instrument(skip(stdin), err)]
    fn exec(
        &self,
        command: &Command,
        stdin: Option<&[u8]>,
        rlimits: ResourceLimits,
        profile: Profile,
    ) -> io::Result<Output> {
        let mut cmd = process::Command::new(&command.executable);
        cmd.args(&command.args)
            .current_dir(self.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        unsafe {
            let dir = self.path().to_path_buf();
            cmd.pre_exec(move || sandbox(dir.clone(), rlimits, profile));
        }

        let mut child = cmd.spawn()?;

        if let Some(stdin) = stdin {
            child.stdin.take().expect("no stdin").write_all(stdin)?;
        }

        let (stdout, stderr) = {
            let (mut stdout, mut stderr) = (
                child.stdout.take().expect("no stdout"),
                child.stderr.take().expect("no stderr"),
            );
            let (mut stdout_buf, mut stderr_buf) = (Vec::new(), Vec::new());

            stdout.read_to_end(&mut stdout_buf)?;
            stderr.read_to_end(&mut stderr_buf)?;

            (stdout_buf, stderr_buf)
        };

        let (exit_status, resource_usage) = resource::wait4(child.id() as i32)?;

        Ok(Output {
            exit_status,
            stdout,
            stderr,
            resource_usage,
        })
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Profile {
    Build,
    Run,
}

fn sandbox(dir: PathBuf, rlimits: ResourceLimits, profile: Profile) -> io::Result<()> {
    use io::{Error, ErrorKind};

    if let Profile::Run = profile {
        landlock::restrict_thread(dir).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    }

    rlimits.set()?;

    if let Profile::Run = profile {
        seccomp::apply_filters().map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    }

    Ok(())
}
