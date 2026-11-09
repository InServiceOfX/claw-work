import importlib.util
import pathlib
import unittest

spec = importlib.util.spec_from_file_location("benchmark", pathlib.Path(__file__).parents[1] / "benchmark.py")
benchmark = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmark)

class GradingTests(unittest.TestCase):
    def test_primes_reject_extra_or_explanation(self):
        good = ",".join(map(str, benchmark.PRIMES))
        self.assertTrue(benchmark.valid_primes(good))
        self.assertFalse(benchmark.valid_primes(good + ",73"))
        self.assertFalse(benchmark.valid_primes("Primes: " + good))

    def test_rust_requires_logic_and_reports_missing_tests(self):
        wrong = "fn merge(a: &[i32], b: &[i32]) -> Vec<i32> { [a, b].concat() }"
        self.assertFalse(benchmark.rust_check(wrong)["logic"])
        valid = "fn merge(a: &[i32], b: &[i32]) -> Vec<i32> { let mut out = [a, b].concat(); out.sort(); out }"
        result = benchmark.rust_check(valid)
        self.assertTrue(result["logic"])
        self.assertFalse(result["included_tests"])
        bad_authored_test = valid + "\n#[test] fn incorrect_expectation() { assert_eq!(merge(&[1], &[2]), vec![2, 1]); }"
        result = benchmark.rust_check(bad_authored_test)
        self.assertTrue(result["logic"])
        self.assertTrue(result["included_tests"])
        self.assertFalse(result["authored_tests_pass"])
        documented = "```rust\n/// ```rust\n/// example();\n/// ```\n" + valid + "\n#[test] fn authored() { assert_eq!(merge(&[1], &[2]), vec![1, 2]); }\n```"
        result = benchmark.rust_check(documented)
        self.assertTrue(result["logic"])
        self.assertTrue(result["authored_tests_pass"])
        broken = "fn merge(a: &[i32], b: &[i32]) -> Vec<i32> { let mut out = a.to_vec(); out.extend_slice(b); out }"
        self.assertFalse(benchmark.rust_check(broken)["logic"])

if __name__ == "__main__":
    unittest.main()
