//! Kernel-accounted RSS, never a sampling approximation or allocator byte count.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Reading {
    pub current_bytes: u64,
    pub lifetime_peak_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct Interval {
    pub baseline: Reading,
    pub end: Reading,
    pub peak_increment_bytes: Option<u64>,
    pub proof: &'static str,
}

impl Interval {
    pub fn between(baseline: Reading, end: Reading) -> Result<Self, String> {
        if baseline.lifetime_peak_bytes < baseline.current_bytes
            || end.lifetime_peak_bytes < end.current_bytes
            || end.lifetime_peak_bytes < baseline.lifetime_peak_bytes
        {
            return Err("inconsistent or regressing kernel RSS counters".into());
        }
        let proof = if baseline.lifetime_peak_bytes == baseline.current_bytes {
            "baseline_at_lifetime_peak"
        } else if end.lifetime_peak_bytes > baseline.lifetime_peak_bytes {
            "new_lifetime_peak_in_interval"
        } else {
            "ambiguous_prior_peak"
        };
        let peak_increment_bytes = if proof == "ambiguous_prior_peak" {
            None
        } else {
            Some(end.lifetime_peak_bytes - baseline.current_bytes)
        };
        Ok(Self {
            baseline,
            end,
            peak_increment_bytes,
            proof,
        })
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // ADR-0020: confined to the measurement binary, never core.
pub fn read() -> Result<Reading, String> {
    // SAFETY: libc supplies the SDK ABI layout (including packed alignment),
    // flavor and count. Zero is a valid bit pattern for all of this C struct's
    // scalar fields. The writable buffer remains alive across the synchronous
    // call; the kernel receives its exact capacity in natural_t units. Only our
    // own task port is queried. No foreign pointer or untrusted PID is accepted.
    let mut info: libc::mach_task_basic_info = unsafe { std::mem::zeroed() };
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    // libc discourages its Mach bindings in favor of a separate crate, but
    // this self-port ABI is still in the installed SDK. Reuse the locked libc
    // instead of adding another native dependency for this isolated tool.
    #[expect(
        deprecated,
        reason = "ADR-0020 reuses locked libc for the SDK self-task port"
    )]
    let task = unsafe { libc::mach_task_self() };
    let result = unsafe {
        libc::task_info(
            task,
            libc::MACH_TASK_BASIC_INFO,
            std::ptr::from_mut(&mut info).cast::<libc::integer_t>(),
            &raw mut count,
        )
    };
    if result != libc::KERN_SUCCESS || count != libc::MACH_TASK_BASIC_INFO_COUNT {
        return Err(format!("task_info failed: code={result}, count={count}"));
    }
    // Copies, not references into potentially unaligned packed fields.
    Ok(Reading {
        current_bytes: info.resident_size,
        lifetime_peak_bytes: info.resident_size_max,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn read() -> Result<Reading, String> {
    Err("peak RSS instrumentation is supported on macOS only".into())
}

/// Diagnostic-only mapped pages. Unlike malloc/free, munmap cannot leave the
/// allocation in a userspace allocator cache, so a release test is meaningful.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // ADR-0020: fixed-size diagnostic mapping, never a workload allocator.
pub fn probe_pages(observe_mapped: bool, observe: &mut dyn FnMut()) -> Result<(), String> {
    const BYTES: usize = 64 * 1024 * 1024;
    // SAFETY: ask the OS for a new anonymous private read/write mapping. No
    // caller address/fd/length is accepted, and MAP_FAILED is checked before
    // access. write_bytes touches the exact valid range, making it resident.
    let pages = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            BYTES,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        )
    };
    if pages == libc::MAP_FAILED {
        return Err(format!("probe mmap: {}", std::io::Error::last_os_error()));
    }
    unsafe {
        std::ptr::write_bytes(pages.cast::<u8>(), 0xA5, BYTES);
    }
    std::hint::black_box(pages);
    if observe_mapped {
        // The only caller supplies the fixed read-only observation closure.
        // A panic terminates this diagnostic child (OS reclaims the mapping).
        observe();
    }
    // SAFETY: exact base/length from the successful mmap above. No pointer
    // escapes or is dereferenced after unmapping; errors fail the probe.
    if unsafe { libc::munmap(pages, BYTES) } != 0 {
        return Err(format!("probe munmap: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn probe_pages(_observe_mapped: bool, _observe: &mut dyn FnMut()) -> Result<(), String> {
    Err("peak RSS diagnostic mapping is supported on macOS only".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(current_bytes: u64, lifetime_peak_bytes: u64) -> Reading {
        Reading {
            current_bytes,
            lifetime_peak_bytes,
        }
    }

    #[test]
    fn baseline_at_peak_proves_zero_and_released_peak() {
        let zero = Interval::between(sample(100, 100), sample(80, 100)).unwrap();
        assert_eq!(zero.peak_increment_bytes, Some(0));
        let released = Interval::between(sample(100, 100), sample(80, 500)).unwrap();
        assert_eq!(released.peak_increment_bytes, Some(400));
    }

    #[test]
    fn new_peak_proves_interval_even_after_earlier_allocation() {
        let interval = Interval::between(sample(100, 500), sample(90, 600)).unwrap();
        assert_eq!(interval.peak_increment_bytes, Some(500));
        assert_eq!(interval.proof, "new_lifetime_peak_in_interval");
    }

    #[test]
    fn old_peak_cannot_be_subtracted_or_reported_as_zero() {
        let interval = Interval::between(sample(100, 500), sample(110, 500)).unwrap();
        assert_eq!(interval.peak_increment_bytes, None);
        assert_eq!(interval.proof, "ambiguous_prior_peak");
    }

    #[test]
    fn impossible_or_regressing_readings_fail() {
        for (before, after) in [
            (sample(101, 100), sample(200, 200)),
            (sample(100, 100), sample(201, 200)),
            (sample(100, 200), sample(100, 199)),
        ] {
            assert!(Interval::between(before, after).is_err());
        }
    }

    #[test]
    fn full_u64_range_is_checked_without_wrapping() {
        let interval = Interval::between(sample(0, 0), sample(0, u64::MAX)).unwrap();
        assert_eq!(interval.peak_increment_bytes, Some(u64::MAX));
    }
}
