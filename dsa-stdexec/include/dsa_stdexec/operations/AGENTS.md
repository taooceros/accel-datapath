# operations AGENTS

Inherits `../../../AGENTS.md`.

## OVERVIEW
Per-operation sender implementations. Each operation follows the same sender/state/completion pattern.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Shared operation pattern | `README.md` | Local checklist is authoritative. |
| Shared mixin | `operation_base_mixin.hpp` | Common lifecycle logic. |
| Operation registry header | `all.hpp` | Must include every public op header. |
| Descriptor helpers | `../descriptor_fill.hpp` | Reuse before inventing new fill code. |
| Example shape | `../../../examples/README.md` | One example per op. |
| Build registration | `../../../xmake.lua` | `example_<op>` targets live here. |

## CONVENTIONS
- Follow the documented 3-part pattern: `<Op>Operation`, `<Op>Sender`, and `dsa_<op>(...)` factory.
- Use existing operations such as `data_move.hpp` as the template.
- Keep completion handling explicit: success, error, and page-fault retry behavior must be deliberate.

## ANTI-PATTERNS
- Do not add an op header without updating `all.hpp`.
- Do not add a new op without adding or updating an example and `xmake.lua` target registration.
- Do not bypass descriptor helper utilities when an existing fill pattern already matches.
