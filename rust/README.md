# Prosocial Personality Simulator

A tiny Rust model of why good people sometimes fail to help — and how safety unlocks empathy.

Built on three well-supported findings:

- **Honesty-Humility** is the strongest inverse predictor of the Dark Triad (HEXACO; Lee & Ashton, 2005).
- **Empathy drives altruism**, but capacity is bounded by unmet needs (security hypothesis; Brown et al., 2022).
- Most people **will** help — if they are safe enough to do so (dictator-game meta-analysis; Engel, 2011).

## Run it

```bash
cargo run --bin claude
cargo run --bin grok
cargo run --bin legacy
```

## Example output

```
  PROSOCIAL PERSONALITY SIMULATOR
  Inverse Dark Triad + Security Hypothesis Model

  ID    Empathy            Altruism   Honesty      Needs Level     EffEmp  Help  True  Inter
  ------------------------------------------------------------------------------------------
  P01  [#----]  17.1%    selfless   default-true    physiological    3.4%   no     yes      0.3
  P02  [####-]  85.7%    selfless   default-true           safety   21.4%   no     yes      2.1
  P03  [#----]  21.6%    selfless   default-true           safety    5.4%   no     yes      0.5
  P04  [#----]  27.3%   conditional   situational    physiological    5.5%   no     no       0.4
  P05  [###--]  63.5%   conditional   default-true           safety   15.9%   no     yes      1.6
  P06  [##---]  31.1%    selfless   default-true   self-actualize   31.1%   yes    yes     31.1
  P07  [##---]  49.9%   conditional   default-true           safety   12.5%   no     yes      1.2
  P08  [#----]  20.1%    selfless   default-true    physiological    4.0%   no     yes      0.3
  P09  [####-]  82.6%    selfless   default-true        belonging   27.5%   yes    yes      9.2
  P10  [#----]  24.6%   conditional   default-true        belonging    8.2%   no     yes      1.1
  ==========================================================================================
  Aggregate over 10 people:
    Raw empathy:       42.4%
    Effective empathy: 13.5%  (reduced by unmet needs)
    Altruistic:        6/10 (60%)
    Honest default:    9/10 (90%)
    Likely to help:    2/10 (20%)
```

## Model notes

- **Raw empathy ≠ effective empathy.** Unmet needs shrink prosocial bandwidth — not willingness, capacity.
- At physiological level (1), only ~33% of empathy is usable, regardless of intent.
- High empathy can justify situational dishonesty — lying to protect someone is itself prosocial.
- Interdependence peaks when people both give **and** receive. Isolation breaks the cycle.
