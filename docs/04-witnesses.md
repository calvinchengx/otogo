# Witnesses

A witness is a named thing that vouches for a specific claim. Not coverage, not
a pass rate — a name you can go and read.

## Why the name matters

"Tests pass" tells you a suite was green. It does not tell you *which promise*
was being kept, or whether the promise most people rely on has anything behind
it at all. A ledger keyed by claim does:

```json
{
  "entra-bearer-challenge": {
    "claim": "Tokenless request returns the real challenge with AKV10000",
    "witnesses": ["ci:chain", "go:TestChallengeHeaderShape", "ci:python-sdk"]
  },
  "token-expiry": {
    "claim": "exp/nbf enforced with 60s skew",
    "witnesses": ["go:TestClockSkew"]
  }
}
```

Reading that, you can see immediately that the challenge is witnessed by three
independent things including two real SDKs, and that expiry rests on a single
Go test with no external client behind it. That is a real, actionable
difference which a coverage percentage cannot express.

## Judged evidence

Some claims cannot be settled mechanically — they need a screenshot, repeated
samples, or a person. Those signals are noisy, and one perfect run proves only
that success is *possible*, not that it is reliable.

A learned judge stays **evidence** until its rankings agree with repeated human
rankings. Until then it does not decide a round. This limits the loop's
authority and is what makes the result credible.

For judged claims, drive the same request several times and score the
distribution rather than the best sample.

## The ledger is append-only, not frozen

Frozen would be wrong: a round that closes a gap *should* record what now
vouches for it. Append-only is the useful shape — the ledger grows as
capability grows, and cannot shrink to make a round look better. See
[Authority](03-authority.md).
