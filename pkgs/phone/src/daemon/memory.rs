#[cfg(target_os = "linux")]
use anyhow::Context;
use anyhow::Result;

pub const BUDGET: u64 = 32 * 1024 * 1024;

#[cfg(target_os = "linux")]
pub fn resident_bytes() -> Result<u64> {
    let stat = std::fs::read_to_string("/proc/self/statm")?;
    let resident: u64 = stat
        .split_whitespace()
        .nth(1)
        .context("missing resident page count")?
        .parse()?;
    #[allow(unsafe_code, reason = "std does not expose the OS page size")]
    // SAFETY: _SC_PAGESIZE is a valid selector and sysconf takes no pointers.
    // The checked conversion below rejects its negative failure result.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    Ok(resident.saturating_mul(u64::try_from(page_size).context("invalid page size")?))
}

#[cfg(target_os = "macos")]
pub fn resident_bytes() -> Result<u64> {
    let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info_data_t>::zeroed();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    #[allow(
        unsafe_code,
        deprecated,
        reason = "Mach task_info is required for current RSS; libc deprecates the task port accessor"
    )]
    // SAFETY: Flavor and count match the aligned, writable buffer's exact type.
    // The current task port is valid; zeroing initializes every field and padding.
    // The output is read only after task_info reports success.
    unsafe {
        let status = libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast(),
            &mut count,
        );
        if status != libc::KERN_SUCCESS {
            anyhow::bail!("task_info failed with {status}");
        }
        Ok(info.assume_init().resident_size)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn resident_bytes() -> Result<u64> {
    anyhow::bail!("cleanup memory monitoring requires Linux or macOS")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_current_process_resident_memory() {
        assert!(resident_bytes().unwrap() > 0);
    }
}
