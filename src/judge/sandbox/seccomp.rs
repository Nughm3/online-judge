use std::env::consts::ARCH;

use once_cell::sync::Lazy;
use seccompiler::{
    BpfProgram, Result, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition,
    SeccompFilter, SeccompRule,
};

static SECCOMP_FILTER: Lazy<BpfProgram> = Lazy::new(|| {
    let filter = SeccompFilter::new(
        [
            (libc::SYS_access, vec![]),
            (libc::SYS_arch_prctl, vec![]),
            (libc::SYS_brk, vec![]),
            (libc::SYS_clock_getres, vec![]),
            (libc::SYS_clock_gettime, vec![]),
            (libc::SYS_clone, vec![]),
            (libc::SYS_clone3, vec![]),
            (libc::SYS_close, vec![]),
            (libc::SYS_dup, vec![]),
            (libc::SYS_dup2, vec![]),
            (libc::SYS_dup3, vec![]),
            (libc::SYS_epoll_create, vec![]),
            (libc::SYS_epoll_create1, vec![]),
            (libc::SYS_epoll_ctl, vec![]),
            (libc::SYS_epoll_pwait, vec![]),
            (libc::SYS_epoll_wait, vec![]),
            (libc::SYS_execve, vec![]),
            (libc::SYS_exit, vec![]),
            (libc::SYS_exit_group, vec![]),
            (libc::SYS_fcntl, vec![]),
            (libc::SYS_fstat, vec![]),
            (libc::SYS_futex, vec![]),
            (libc::SYS_getcwd, vec![]),
            (libc::SYS_getdents, vec![]),
            (libc::SYS_getdents64, vec![]),
            (libc::SYS_getegid, vec![]),
            (libc::SYS_geteuid, vec![]),
            (libc::SYS_getgid, vec![]),
            (libc::SYS_getpgrp, vec![]),
            (libc::SYS_getpid, vec![]),
            (libc::SYS_getppid, vec![]),
            (libc::SYS_getrandom, vec![]),
            (libc::SYS_getrlimit, vec![]),
            (libc::SYS_getrusage, vec![]),
            (libc::SYS_gettid, vec![]),
            (libc::SYS_gettimeofday, vec![]),
            (libc::SYS_getuid, vec![]),
            (libc::SYS_ioctl, vec![]),
            (libc::SYS_lseek, vec![]),
            (libc::SYS_madvise, vec![]),
            (libc::SYS_mmap, vec![]),
            (libc::SYS_modify_ldt, vec![]),
            (libc::SYS_mprotect, vec![]),
            (libc::SYS_mremap, vec![]),
            (libc::SYS_munmap, vec![]),
            (libc::SYS_newfstatat, vec![]),
            (libc::SYS_nanosleep, vec![]),
            (
                libc::SYS_openat,
                vec![
                    SeccompRule::new(vec![SeccompCondition::new(
                        2,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        libc::O_RDONLY as u64,
                    )
                    .expect("failed to create seccomp condition")])
                    .expect("failed to create seccomp rule"),
                    SeccompRule::new(vec![SeccompCondition::new(
                        2,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        (libc::O_RDONLY | libc::O_CLOEXEC) as u64,
                    )
                    .expect("failed to create seccomp condition")])
                    .expect("failed to create seccomp rule"),
                    SeccompRule::new(vec![SeccompCondition::new(
                        2,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        (libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_DIRECTORY)
                            as u64,
                    )
                    .expect("failed to create seccomp condition")])
                    .expect("failed to create seccomp rule"),
                ],
            ),
            (libc::SYS_pipe, vec![]),
            (libc::SYS_pipe2, vec![]),
            (libc::SYS_poll, vec![]),
            (libc::SYS_ppoll, vec![]),
            (libc::SYS_pread64, vec![]),
            (libc::SYS_read, vec![]),
            (libc::SYS_readlink, vec![]),
            (libc::SYS_readlinkat, vec![]),
            (libc::SYS_restart_syscall, vec![]),
            (libc::SYS_rt_sigaction, vec![]),
            (libc::SYS_rt_sigprocmask, vec![]),
            (libc::SYS_rt_sigreturn, vec![]),
            (libc::SYS_sched_getaffinity, vec![]),
            (libc::SYS_sched_getparam, vec![]),
            (libc::SYS_sched_get_priority_max, vec![]),
            (libc::SYS_sched_get_priority_min, vec![]),
            (libc::SYS_sched_getscheduler, vec![]),
            (libc::SYS_sched_setscheduler, vec![]),
            (libc::SYS_sched_yield, vec![]),
            (libc::SYS_select, vec![]),
            (libc::SYS_set_robust_list, vec![]),
            (libc::SYS_set_thread_area, vec![]),
            (libc::SYS_set_tid_address, vec![]),
            (libc::SYS_sigaltstack, vec![]),
            (libc::SYS_statfs, vec![]),
            (libc::SYS_sysinfo, vec![]),
            (libc::SYS_time, vec![]),
            (libc::SYS_timer_create, vec![]),
            (libc::SYS_timer_delete, vec![]),
            (libc::SYS_timerfd_create, vec![]),
            (libc::SYS_timer_settime, vec![]),
            (libc::SYS_uname, vec![]),
            (libc::SYS_write, vec![]),
            (libc::SYS_writev, vec![]),
        ]
        .into(),
        SeccompAction::Errno(1),
        SeccompAction::Allow,
        ARCH.try_into().expect("unsupported architecture"),
    )
    .expect("failed to create seccomp filter");

    filter.try_into().expect("failed to compile seccomp filter")
});

pub fn apply_filters() -> Result<()> {
    seccompiler::apply_filter(&SECCOMP_FILTER)
}
