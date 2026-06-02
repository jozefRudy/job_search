---
name: retrospect
description: Post-session retrospective. Use after finishing a coding session to analyze failures, inefficiencies, and design decisions. Add lessons to APPEND_SYSTEM.md.
---

# Retrospect

Run after each session. Goal: extract one lesson and update `.pi/APPEND_SYSTEM.md`.

## Retrospect Workflow

1. Review session transcript (or recall from memory)
2. Answer each section below honestly
3. Write summary
4. If lesson could repeat, add actionable rule to `.pi/APPEND_SYSTEM.md`
5. If no lesson worth persisting, state "No changes needed" and stop

---

## 1. What Failed

| Question | Answer |
|----------|--------|
| Compile errors? | What caused them — rushed edits, missed references, type mismatch? |
| Test failures? | Did `cargo test --lib` hide integration failures? |
| Runtime panics/unwraps? | Should have been `Result` or `Option`? |
| Wrong approach discovered late? | Had to throw away code? |
| Tool call failures? | `edit` mismatches, `bash` wrong commands? |

**Pattern to watch:** Same failure type repeating across sessions = missing rule.

---

## 2. What Was Inefficient

| Question | Answer |
|----------|--------|
| Too many `read`/`grep` cycles? | Sign of unclear codebase or jumping between files without plan |
| Repeated `edit` failures? | Sign of not reading full context before editing |
| `bash` command retries? | Sign of wrong command or missing setup docs |
| Over-engineered solution? | Started simple then added complexity? Or started complex? |
| Duplicated existing logic? | Reimplemented what `SeiParser` or another module already does? |
| Long debugging loop? | Could a test or print have shortened it? |

**Pattern to watch:** Tool call count > 30 for small task = process problem.

---

## 3. What Design Decision Needs Reconsideration

| Question | Answer |
|----------|--------|
| Added `Arc`/`Mutex`/`RefCell`? | Was simpler ownership possible? |
| Added generic `<T>` or complex tuple? | Would concrete type be clearer? |
| New crate added? | Could std lib or existing dep handle it? |
| Infrastructure choice? | Assumed paid node when free RPC works? |
| API shape? | Flat pricing model respected? Push vs pull correct? |

---

## 4. Testing Discipline

| Question | Answer |
|----------|--------|
| Ran full `cargo test`? | Or only `--lib` and missed integration tests? |
| Added regression test for bug? | Or only manual verification? |
| Test name describes behavior? | Not just `test_foo`? |
| Test data realistic? | Using actual blockchain data or toy examples? |

---

## Output Format

```
Session: <topic>
Duration: <approximate>
Failures:
  - <failure> → <root cause>
Inefficiencies:
  - <inefficiency> → <what to do instead>
Design concerns:
  - <decision> → <reconsideration>
Changes made:
  - Updated `.pi/APPEND_SYSTEM.md`: <rule added>
  - No changes needed
```
