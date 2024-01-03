use std::{io, os::unix::process::ExitStatusExt, process::ExitStatus};

use rlimit::{setrlimit, Resource};
use serde::Deserialize;
use time::Duration;

const RLIMIT_CPU_TOLERANCE: u64 = 1;
const RLIMIT_DATA_TOLERANCE: u64 = 1000;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ResourceUsage {
    pub user_time: Duration,
    pub sys_time: Duration,
    pub memory_bytes: u64,
}

impl ResourceUsage {
    pub fn total_time(&self) -> Duration {
        self.user_time + self.sys_time
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub cpu_seconds: u64,
    pub memory_bytes: u64,
}

impl ResourceLimits {
    pub fn set(&self) -> io::Result<()> {
        setrlimit(
            Resource::CPU,
            self.cpu_seconds,
            self.cpu_seconds + RLIMIT_CPU_TOLERANCE,
        )?;
        setrlimit(
            Resource::DATA,
            self.memory_bytes,
            self.memory_bytes + RLIMIT_DATA_TOLERANCE,
        )?;
        Ok(())
    }
}

pub fn wait4(pid: i32) -> io::Result<(ExitStatus, ResourceUsage)> {
    let mut status = 0;
    let mut rusage = std::mem::MaybeUninit::zeroed();

    let result = unsafe { libc::wait4(pid, &mut status, 0, rusage.as_mut_ptr()) };

    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        let rusage = unsafe { rusage.assume_init() };
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as i64;

        Ok((
            ExitStatus::from_raw(status),
            ResourceUsage {
                user_time: timeval_to_duration(rusage.ru_utime),
                sys_time: timeval_to_duration(rusage.ru_stime),
                memory_bytes: (rusage.ru_maxrss * page_size / 2) as u64, // FIXME
            },
        ))
    }
}

#[allow(clippy::useless_conversion)]
fn timeval_to_duration(timeval: libc::timeval) -> Duration {
    let v = i64::from(timeval.tv_sec) * 1_000_000 + i64::from(timeval.tv_usec);
    Duration::microseconds(v)
}
