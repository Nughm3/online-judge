use std::env::consts::ARCH;

use libc::*;
use once_cell::sync::Lazy;
use seccompiler::{
    BpfProgram, Result, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition,
    SeccompFilter, SeccompRule,
};

static SECCOMP_FILTER: Lazy<BpfProgram> = Lazy::new(|| {
    let filter = SeccompFilter::new(
        [
            (SYS_access, vec![]),
            (SYS_arch_prctl, vec![]),
            (SYS_brk, vec![]),
            (SYS_clock_getres, vec![]),
            (SYS_clock_gettime, vec![]),
            (SYS_clone3, vec![]),
            (SYS_clone, vec![]),
            (SYS_close, vec![]),
            (SYS_dup2, vec![]),
            (SYS_dup3, vec![]),
            (SYS_dup, vec![]),
            (SYS_epoll_create1, vec![]),
            (SYS_epoll_create, vec![]),
            (SYS_epoll_ctl, vec![]),
            (SYS_epoll_pwait, vec![]),
            (SYS_epoll_wait, vec![]),
            (SYS_execve, vec![]),
            (SYS_exit_group, vec![]),
            (SYS_exit, vec![]),
            (SYS_fcntl, vec![]),
            (SYS_fstat, vec![]),
            (SYS_futex, vec![]),
            (SYS_getcwd, vec![]),
            (SYS_getdents64, vec![]),
            (SYS_getdents, vec![]),
            (SYS_getegid, vec![]),
            (SYS_geteuid, vec![]),
            (SYS_getgid, vec![]),
            (SYS_getpgrp, vec![]),
            (SYS_getpid, vec![]),
            (SYS_getppid, vec![]),
            (SYS_getrandom, vec![]),
            (SYS_getrlimit, vec![]),
            (SYS_getrusage, vec![]),
            (SYS_gettid, vec![]),
            (SYS_gettimeofday, vec![]),
            (SYS_getuid, vec![]),
            (SYS_ioctl, vec![]),
            (SYS_lseek, vec![]),
            (SYS_madvise, vec![]),
            (SYS_mmap, vec![]),
            (SYS_modify_ldt, vec![]),
            (SYS_mprotect, vec![]),
            (SYS_mremap, vec![]),
            (SYS_munmap, vec![]),
            (SYS_nanosleep, vec![]),
            (SYS_newfstatat, vec![]),
            (
                SYS_open,
                vec![
                    SeccompRule::new(vec![SeccompCondition::new(
                        1,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        O_RDONLY as u64,
                    )
                    .unwrap()])
                    .unwrap(),
                    SeccompRule::new(vec![SeccompCondition::new(
                        1,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        (O_RDONLY | O_LARGEFILE | O_CLOEXEC) as u64,
                    )
                    .unwrap()])
                    .unwrap(),
                ],
            ),
            (
                SYS_openat,
                vec![
                    SeccompRule::new(vec![SeccompCondition::new(
                        2,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        O_RDONLY as u64,
                    )
                    .unwrap()])
                    .unwrap(),
                    SeccompRule::new(vec![SeccompCondition::new(
                        2,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        (O_RDONLY | O_CLOEXEC) as u64,
                    )
                    .unwrap()])
                    .unwrap(),
                    SeccompRule::new(vec![SeccompCondition::new(
                        2,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        (O_RDONLY | O_NONBLOCK | O_CLOEXEC | O_DIRECTORY) as u64,
                    )
                    .unwrap()])
                    .unwrap(),
                ],
            ),
            (SYS_pipe2, vec![]),
            (SYS_pipe, vec![]),
            (SYS_poll, vec![]),
            (SYS_ppoll, vec![]),
            (SYS_pread64, vec![]),
            (SYS_readlinkat, vec![]),
            (SYS_readlink, vec![]),
            (SYS_read, vec![]),
            (SYS_restart_syscall, vec![]),
            (SYS_rseq, vec![]),
            (SYS_rt_sigaction, vec![]),
            (SYS_rt_sigprocmask, vec![]),
            (SYS_rt_sigreturn, vec![]),
            (SYS_sched_getaffinity, vec![]),
            (SYS_sched_getparam, vec![]),
            (SYS_sched_get_priority_max, vec![]),
            (SYS_sched_get_priority_min, vec![]),
            (SYS_sched_getscheduler, vec![]),
            (SYS_sched_setscheduler, vec![]),
            (SYS_sched_yield, vec![]),
            (SYS_select, vec![]),
            (SYS_set_robust_list, vec![]),
            (SYS_set_thread_area, vec![]),
            (SYS_set_tid_address, vec![]),
            (SYS_sigaltstack, vec![]),
            (SYS_statfs, vec![]),
            (SYS_sysinfo, vec![]),
            (SYS_timer_create, vec![]),
            (SYS_timer_delete, vec![]),
            (SYS_timerfd_create, vec![]),
            (SYS_timer_settime, vec![]),
            (SYS_time, vec![]),
            (SYS_uname, vec![]),
            (SYS_write, vec![]),
            (SYS_writev, vec![]),
        ]
        .into(),
        SeccompAction::Errno(EPERM as u32),
        SeccompAction::Allow,
        ARCH.try_into().expect("unsupported architecture"),
    )
    .expect("failed to create seccomp filter");

    filter.try_into().expect("failed to compile seccomp filter")
});

pub fn apply_filters() -> Result<()> {
    seccompiler::apply_filter(&SECCOMP_FILTER)
}
