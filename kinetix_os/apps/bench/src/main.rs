#![no_std]
#![no_main]

use core::hint::black_box;
use kinetix_runtime::{exit, print_str, print_u64, uptime_ms, yield_now};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print_str("  [APP: bench] Starting user-space arithmetic benchmark...\n");

    let t0 = uptime_ms();
    let mut sum: u64 = 0;
    let mut primes_found: u64 = 0;

    // Search for primes in 2..3000 with cooperative yielding
    for n in 2..3000 {
        let mut is_prime = true;
        let mut d = 2;
        while d * d <= n {
            if n % d == 0 {
                is_prime = false;
                break;
            }
            d += 1;
        }
        if is_prime {
            primes_found += 1;
            sum = sum.wrapping_add(n);
        }

        // Cooperatively yield every 500 iterations
        if n % 500 == 0 {
            yield_now();
        }
    }

    black_box(sum);
    let t1 = uptime_ms();

    print_str("  [APP: bench] Search complete (range: 2..3000).\n");
    print_str("  [APP: bench] Primes discovered  : ");
    print_u64(primes_found);
    print_str("\n");
    print_str("  [APP: bench] Prime sum check    : ");
    print_u64(sum);
    print_str("\n");
    print_str("  [APP: bench] Benchmark duration : ");
    print_u64(t1.saturating_sub(t0));
    print_str(" ms\n");
    print_str("  [APP: bench] Benchmark completed successfully with status 0.\n");

    exit(0);
}
