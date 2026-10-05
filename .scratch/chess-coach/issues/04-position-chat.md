# 04: Chat about the current position

**What to build:** A chat beside the board streams LLM answers (plain pi-ai, github-copilot provider, behind one function). For the viewed position the prompt holds only checked data: FEN, piece squares, played and best move in SAN, evaluation, top lines, the punishing line, plus player rating and time control when known. The model is told to explain only from that data and say it is not sure otherwise. Answer quality is not judged in this ticket.

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] Answers stream into the chat
- [ ] Prompt data refreshes when the viewed position changes
- [ ] Rating and time control are stored from the PGN and sent when present
- [ ] A clear error shows if the LLM is unavailable and the board still works
- [ ] Tests check the prompt holds only validated data, using a fake provider
