# 0009 — World hi-score board: stage log to the owner's score server, sent with the OS `curl`

Status: accepted (2026-11-19)

## Context
The local hi score should be kept, and there should be a world board (top 1000) ranked by kills and speed. The game is
open source, so nothing in it can be secret for real, and it has no HTTP/TLS crates (ADR 0001: Bevy only).

## Decision
- **Score = kills + power-ups + a time bonus per cleared classic stage** (`rules::time_bonus`, integer tenths so the
  server computes the same number), added in play and shown in the results tally.
- At game over (classic) each player enters 3 letters; the game sends the **whole stage log** (time, kills per type,
  power-ups, per stage and player), not only the total, to the owner's multi-game score server
  (`scores.maxpodolski.com`, game id `battletanks`). The server recomputes the score and refuses impossible runs.
- **Transport: the OS `curl`** (in Windows 10+ and every desktop Linux/macOS) on a worker thread, behind
  `Platform::post_score` so a host can swap it or turn it off (`None`). The answer is plain text (`?format=text`) so the
  game needs no JSON parser.
- **Game key** in an `X-Game-Key` header, baked in at build time from the gitignored `scores.key` (`build.rs` →
  `BT_SCORES_KEY`). Builds without it are offline; forks can't post by accident.
- Local best and last initials in `scores.txt` next to the settings.

## Alternatives
- `ureq`/`reqwest` + rustls: one code path for every platform, but a TLS stack and ~1–2 MB for one POST per game over.
- Sending just the total: trivially faked; the log lets the server check it against the rules.
- Signing the body with the key (HMAC): the key is in the exe either way.

## Consequences
- Scoring changes touch `rules.rs` and the server's `RunRules` together (shared test vector on both sides).
- A PC without `curl` stays offline (shown as "offline - score not sent"); Android TV is offline by choice (no
  internet permission, Play listing says it sends nothing).
- Someone who extracts the key and fakes a plausible log can still get on the board; the server owner removes it.
