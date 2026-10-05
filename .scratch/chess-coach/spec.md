Status: ready-for-agent

## Problem Statement

Stockfish evaluations are hard for a human to read. I see a move lose evaluation but cannot tell why, or which line punishes it. I want a coach that explains my mistakes in plain language, based on real engine lines.

## Solution

A personal web app. I import my games (Lichess username or pasted PGN). The app analyses each game with Stockfish and shows a Lichess-style review screen: board, move list, evaluation bar and a streaming chat with an LLM. The LLM always knows the position I am viewing, its evaluation, the best lines and the line that punishes the move played, and explains from that data only.

## User Stories

1. As a player, I want to paste a PGN, so that I can review a game from any source
2. As a player, I want to import my games from Lichess by username, so that I don't copy games by hand
3. As a player, I want to save my username once, so that the app knows which side to coach
4. As a player, I want to pick a side when the names don't match, so that the coaching is for the right player
5. As a player, I want the board to open at once while analysis runs in the background, so that I don't wait
6. As a player, I want the evaluation bar to fill in as analysis finishes, so that I see progress
7. As a player, I want to step through moves with the keyboard and the move list, so that I can follow the game
8. As a player, I want to see the evaluation of each position, so that I know who is better
9. As a player, I want moves labelled inaccuracy, mistake or blunder, so that I can find the key moments
10. As a player, I want labels based on win-probability loss, so that a loss when already winning counts less
11. As a player, I want to jump to the next mistake, so that I can review quickly
12. As a player, I want a chat beside the board, so that I can ask why a move is bad
13. As a player, I want the LLM to know the current position, move and evaluation, so that answers match what I see
14. As a player, I want the LLM to know the top engine lines and the line that punishes my move, so that it explains the cause of the loss
15. As a player, I want answers to stream, so that I read them as they appear
16. As a player, I want one chat per game that remembers history, so that I can refer to earlier moves
17. As a player, I want each message tied to the position where I asked it, so that the history makes sense
18. As a player, I want the coach to say "I'm not sure" instead of inventing facts, so that I can trust it
19. As a player, I want the coach to know my rating and the time control, so that advice fits my level
20. As a player, I want analysis saved, so that reopening a game is instant
21. As a player, I want saved explanations kept, so that I don't pay for the same call twice
22. As a player, I want to play a side variation on the board and get an on-demand evaluation, so that I can test ideas
23. As a player, I want a clear error if the LLM is unavailable, so that the board still works
24. As a developer, I want the LLM behind one function, so that I can swap the provider

## Implementation Decisions

- Rust backend owns all chess work: PGN parsing, move validation, running Stockfish as a child process, SQLite storage, and a small HTTP API.
- Stockfish: depth 18, 3 lines per position, run as a background job per game. Results stored per position.
- Mistake labels use win-probability loss, starting from Lichess's published cut-offs; tune later.
- Frontend is SvelteKit with TypeScript. Its Node server routes hold the LLM layer.
- LLM layer: calls `pi-ai` with the `github-copilot` provider and streams replies. It fetches checked position data from the Rust API per message and builds the prompt. No chess logic lives in it.
- Prompt data per position: FEN, piece squares, played move and best move in SAN, evaluation, top lines (about 8 moves), punishing line, player rating, time control. The prompt tells the model to explain only from this data.
- One chat per game; messages are tagged with the position they were asked at.
- Pi Durable is not used in this version; the LLM call sits behind one function so it can be added later.
- Seed the glossary with: position, mistake, punishing line, review.

## Testing Decisions

- Good tests check external behaviour only: input in, output out, never internals.
- Main seam: the Rust HTTP API. Post a PGN, then check stored analysis, labels and the checked position data. Use a fake Stockfish with fixed output for speed. Add a few tests against the real engine.
- Second seam: the LLM function. Check that the prompt holds only validated data. Use a fake provider.
- Svelte screens are not tested in the first version.
- No prior art; the repo is new.

## Out of Scope

- Cross-game "what to work on" reports.
- Clock-based time-management advice (keep the clock data in storage if easy).
- Clickable lines in chat messages.
- Pi Durable and crash recovery.
- Multi-user accounts, hosting, and other import sources.

## Further Notes

- Open risk: Copilot's terms may not allow use from your own code. I could not confirm this.
- Open risk: LLM explanation quality on engine lines is unproven; the first slice exists to test it.
