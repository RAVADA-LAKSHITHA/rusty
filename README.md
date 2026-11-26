# 🦀 Rusty

A terminal chatbot in Rust. Answers any question via an LLM (Groq by default, any OpenAI-compatible API works) and falls back to offline small talk when no key is set.

## Run
```bash
export GROQ_API_KEY=gsk_...      # free key at console.groq.com
cargo run --release              # interactive chat ('reset', 'exit')
cargo run --release -- "What is a monad?"   # one-shot
```
Optional: `RUSTY_API_URL`, `RUSTY_API_KEY`, `RUSTY_MODEL` to use another provider/model.

## GitHub automation
| Workflow | Trigger | Does |
|---|---|---|
| `ci.yml` | push / PR | fmt, clippy, tests |
| `release.yml` | tag `v*` | builds Linux/Windows/macOS binaries, attaches to a Release |
| `ask-rusty.yml` | issue labelled `ask`, comment `/rusty ...`, or manual run | Rusty answers as an issue comment |

Setup: repo **Settings → Secrets → Actions** → add `GROQ_API_KEY`; create an `ask` label.
Release: `git tag v0.1.0 && git push --tags`.
