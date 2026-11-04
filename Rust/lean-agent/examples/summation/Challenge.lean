import Mathlib.Tactic

def iterativeSum : Nat → Nat
  | 0 => 0
  | n + 1 => iterativeSum n + n + 1

def iterativeCost (n : Nat) : Nat := n
def closedCost (_ : Nat) : Nat := 3

theorem sum_formula (n : Nat) : iterativeSum n = n * (n + 1) / 2 := by sorry

theorem modeled_cost_improves (n : Nat) (h : 3 ≤ n) : closedCost n ≤ iterativeCost n := by sorry
