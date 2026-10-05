# 01: Paste a PGN and see the board

**What to build:** Paste a PGN and see the game on a board with a move list. A Rust service parses and validates the game and stores it in SQLite. A basic Svelte screen lets you step through the moves with the mouse and keyboard.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] A pasted PGN is stored and reopens later
- [x] Board and move list stay in sync while stepping
- [x] Invalid PGN shows a clear error
- [x] Tests on the Rust HTTP API
