import Mathlib.Tactic

def badWithdraw (balance amount : Nat) : Option Nat :=
  if amount < balance then some (balance - amount) else none

def repairedWithdraw (balance amount : Nat) : Option Nat :=
  if amount ≤ balance then some (balance - amount) else none

theorem boundary_counterexample : badWithdraw 5 5 = none ∧ repairedWithdraw 5 5 = some 0 := by decide

theorem buggy_contract_false : ¬ (∀ b a : Nat, a ≤ b → badWithdraw b a = some (b - a)) := by
  intro h
  have bad := h 5 5 (by decide)
  simp [badWithdraw] at bad

theorem repaired_contract : ∀ b a : Nat, repairedWithdraw b a = some (b - a) ↔ a ≤ b := by
  intro b a
  by_cases h : a ≤ b <;> simp [repairedWithdraw, h]
