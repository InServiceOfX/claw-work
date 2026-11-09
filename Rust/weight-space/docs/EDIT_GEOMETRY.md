# What the weight edit does mathematically

This note explains the operator family. The first GPU experiment replays exact
released BF16 replacements; it does not estimate a new direction or choose new
neurons. The distinction matters for reproducibility and novelty claims.

Let an MLP output projection have matrix `W` with `d_out` rows and `d_in` columns.
For an input activation `h`, its residual-stream contribution is `W h`. Let `u`
be a unit vector in the residual output space. Let diagonal mask `M` contain one
for each selected input neuron and zero elsewhere. Consider

```text
W' = W - alpha u (u^T W M).                                  (1)
```

The matrix `u (u^T W M)` has rank at most one before quantization. Its rows and
columns are not interchangeable: `u` has length `d_out`, and the selection mask
acts on the `d_in` input columns. A transposed storage convention requires the
transposed operator. This family appears in
[ThinkEdit §4.3](https://arxiv.org/html/2503.22048v4#S4.SS3); its application there
targets short-reasoning heads, which is a different behavioral objective.

## Projection and minimum change

For a selected column `w_j`, decompose it as

```text
w_j = u (u^T w_j) + v_j,       u^T v_j = 0.                  (2)
```

At `alpha = 1`, substituting (2) into (1) leaves `w'_j = v_j`: that column cannot
write along `u`. Unselected columns remain exactly unchanged in real arithmetic.

This is also the unique minimum-Euclidean-change solution for that column under
the constraint `u^T w'_j = 0`. To see why, write any proposed change as
`delta_j = c u + z`, with `u^T z = 0`. The constraint forces
`c = -u^T w_j`, while orthogonality gives

```text
||delta_j||_2^2 = (u^T w_j)^2 + ||z||_2^2.                    (3)
```

The minimum occurs at `z = 0`. Applying the same argument to each selected column
gives the minimum-Frobenius-change matrix satisfying those column constraints.
This is a local linear-algebra statement, not a guarantee of minimum behavioral
disruption in the nonlinear network.

For arbitrary alpha, the selected projection becomes
`u^T w'_j = (1-alpha) u^T w_j`. Alpha between zero and one attenuates it; alpha
greater than one reverses it rather than merely deleting it. Replacing `u` by
`-u` leaves the operator unchanged.

## Why a permanent edit differs from fixed activation addition

Multiplying (1) by the current input activation gives

```text
W'h = Wh - alpha u (u^T W M h).                              (4)
```

The intervention's magnitude depends on `h`: selected neurons that are inactive
contribute nothing to the change. A fixed runtime addition such as `Wh + beta u`
has a different dependence on the input. A successful activation-addition control
therefore motivates, but does not prove, the corresponding weight intervention.

For a future discovery experiment, a contrastive mean difference is one way to
propose `u`, but it needs a separate causal check. A direction might reflect
format, token identity, language, response length or dataset selection rather
than a mechanism unique to the intended behavior. See
[CAA §3](https://arxiv.org/html/2312.06681v4#S3) for paired activation extraction.

## Sparse parameter counts and numerical size

If `k` input columns are selected, at most `d_out * k` entries are touched in one
matrix. That can be a tiny fraction of an entire model while still substantially
changing those columns. In exact arithmetic,

```text
||W' - W||_F = |alpha| ||u^T W M||_2.                        (5)
```

The released loop patch selects 64 columns in each of two 1,024-row matrices:
`2 * 64 * 1024 = 131072` touched values. After BF16 rounding, 130,188 entries
actually differ. Their changes have relative Frobenius norms of approximately
10.64% and 8.11% **within the two full target matrices**. The small global entry
fraction does not imply a negligible local perturbation.

## BF16 and exact replay

BF16 quantization can round a touched value back to its original bit pattern.
It can also leave a nonzero component along `u` after an ideal projection. The
rounded difference matrix need not remain exactly rank one, even when the
unrounded update was rank one. For this reason, count changed entries after the
cast, and verify finite values after conversion.

The public artifact already specifies exact replacement bits and hashes for
the evaluated BF16 tensors. The Rust replay tool uses those bits. Recomputing an
approximate rank-one edit from a reconstructed vector could be scientifically
interesting, but would be a different numerical experiment and would not justify
claiming the same checkpoint.

The library's standalone projection routine accepts FP32 matrices and directions,
uses wider accumulations for normalization and inner products, and returns FP32.
It checks geometry on finite synthetic matrices. It does not infer the semantic
meaning of a direction, reproduce the original discovery process, or establish
that any newly projected language model will improve.
