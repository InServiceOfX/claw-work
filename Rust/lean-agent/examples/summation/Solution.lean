import Mathlib.Tactic

def iterativeSum : Nat → Nat
  | 0 => 0
  | n + 1 => iterativeSum n + n + 1

def iterativeCost (n : Nat) : Nat := n
def closedCost (_ : Nat) : Nat := 3

theorem double_sum (n : Nat) : 2 * iterativeSum n = n * (n + 1) := by
  induction n with
  | zero => rfl
  | succ n ih =>
    simp only [iterativeSum]
    nlinarith

theorem sum_formula (n : Nat) : iterativeSum n = n * (n + 1) / 2 := by
  have h := double_sum n
  omega

theorem modeled_cost_improves (n : Nat) (h : 3 ≤ n) : closedCost n ≤ iterativeCost n := by
  simpa [closedCost, iterativeCost] using h
