//! Authored synthetic source fixtures; correspondence to Lean is reviewed, not mechanized.
use std::{hint::black_box, time::Instant};

pub fn buggy_withdraw(balance: u64, amount: u64) -> Option<u64> {
    if amount < balance {
        Some(balance - amount)
    } else {
        None
    }
}
pub fn repaired_withdraw(balance: u64, amount: u64) -> Option<u64> {
    balance.checked_sub(amount)
}
pub fn iterative_sum(n: u64) -> u128 {
    let mut sum = 0_u128;
    for i in 1..=n {
        // Preserve iterative work in the benchmark; LLVM can otherwise derive a
        // closed form itself. This is not a claim about an optimized application.
        sum = black_box(sum) + u128::from(i);
    }
    sum
}
pub fn closed_sum(n: u64) -> u128 {
    let n = u128::from(n);
    n * (n + 1) / 2
}
fn main() {
    let n = 1_000_000_u64;
    let start = Instant::now();
    for _ in 0..20 {
        black_box(iterative_sum(black_box(n)));
    }
    let slow = start.elapsed().as_nanos() / 20;
    let start = Instant::now();
    for _ in 0..100_000 {
        black_box(closed_sum(black_box(n)));
    }
    let fast = start.elapsed().as_nanos() / 100_000;
    println!(
        "{}",
        serde_json::json!({
            "fixture":"synthetic_triangular_sum", "n":n,
            "iterative_mean_ns":slow,"closed_mean_ns":fast,
            "iterative_calls":20,"closed_calls":100000,
            "caveat":"Iterative benchmark has an explicit optimization barrier; not an application speedup claim. Lean proves a Nat model, not this compiled Rust program."
        })
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concrete_source_bug_reproducer_and_repair() {
        assert_eq!(buggy_withdraw(5, 5), None);
        assert_eq!(repaired_withdraw(5, 5), Some(0));
        assert_ne!(buggy_withdraw(5, 5), repaired_withdraw(5, 5));
        assert_eq!(repaired_withdraw(5, 6), None);
        assert_eq!(repaired_withdraw(0, 0), Some(0));
    }
    #[test]
    fn derived_formula_agrees_on_tested_machine_values() {
        for n in 0..1000 {
            assert_eq!(iterative_sum(n), closed_sum(n));
        }
        assert_eq!(
            closed_sum(u64::MAX),
            ((u128::from(u64::MAX) + 1) * u128::from(u64::MAX)) / 2
        );
    }
}
