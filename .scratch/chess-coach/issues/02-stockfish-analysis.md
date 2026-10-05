# 02: Background Stockfish analysis

**What to build:** After import, the Rust service analyses every position in the background (depth 18, 3 lines) and stores results. The board opens at once and the evaluation bar fills in as results arrive.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] Evaluations and top lines are stored per position
- [ ] Evaluation bar and per-move evaluation shown in the UI
- [ ] Reopening a game needs no re-analysis
- [ ] Tests with a fake Stockfish, plus a few against the real engine
