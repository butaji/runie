# Minimax live Runie scenario matrix

This matrix is the live-provider acceptance inventory. Scenarios marked `unit`
are covered by deterministic tests; scenarios marked `tmux` require the actual
Runie binary and Minimax credentials. Every tmux scenario must leave the
process alive unless its oracle explicitly requires a clean exit.

| # | Area | Scenario | Oracle | Mode |
|---:|---|---|---|---|
| 1 | startup | launch with valid key | TUI reaches prompt | tmux |
| 2 | startup | launch without key | actionable configuration error | unit |
| 3 | startup | empty key | falls back without secret leakage | unit |
| 4 | startup | custom endpoint | request uses override | unit |
| 5 | startup | custom model | request uses override | unit |
| 6 | startup | invalid endpoint | TUI reports network error | tmux |
| 7 | startup | non-success HTTP | provider message is shown | unit |
| 8 | startup | malformed JSON | API error is shown | unit |
| 9 | startup | missing choices | no panic; error/empty completion | unit |
| 10 | startup | empty content | completion remains stable | unit |
| 11 | basic | one plain prompt | exact answer renders | tmux |
| 12 | basic | second turn | context continues | tmux |
| 13 | basic | three consecutive turns | all turns settle | tmux |
| 14 | basic | empty prompt | no provider request | tmux |
| 15 | basic | whitespace prompt | no crash | tmux |
| 16 | basic | Unicode prompt | Unicode survives round trip | tmux |
| 17 | basic | emoji prompt | cell width remains stable | tmux |
| 18 | basic | long prompt | wraps without panic | tmux |
| 19 | basic | multiline prompt | lines preserve order | tmux |
| 20 | basic | repeated identical prompt | both turns remain visible | tmux |
| 21 | markdown | ATX heading | heading is bold/styled | tmux |
| 22 | markdown | unordered list | bullets are projected | tmux |
| 23 | markdown | ordered list | ordered items are projected | tmux |
| 24 | markdown | bold | emphasis style applies | tmux |
| 25 | markdown | italic | emphasis style applies | tmux |
| 26 | markdown | inline code | code styling applies | tmux |
| 27 | markdown | link | link text remains readable | tmux |
| 28 | markdown | blockquote | quote gutter renders | tmux |
| 29 | markdown | fenced code | fence/code styling renders | tmux |
| 30 | markdown | syntax language fence | language marker survives | tmux |
| 31 | markdown | simple table | box borders render | tmux |
| 32 | markdown | wide table | table wraps or scrolls safely | tmux |
| 33 | markdown | aligned table | alignment markers do not leak | tmux |
| 34 | markdown | table with Unicode | widths remain aligned | tmux |
| 35 | markdown | adjacent paragraphs | spacing is preserved | tmux |
| 36 | markdown | mixed markdown | styles do not interfere | tmux |
| 37 | markdown | incomplete fence | no panic while streaming | tmux |
| 38 | markdown | incomplete table | no panic while streaming | tmux |
| 39 | markdown | literal pipes | non-table pipes remain text | unit |
| 40 | markdown | escaped markers | escaped syntax remains literal | unit |
| 41 | streaming | delayed response | waiting state is visible | tmux |
| 42 | streaming | reasoning response | thought row settles | tmux |
| 43 | streaming | text after reasoning | final text follows thought | tmux |
| 44 | streaming | very short response | no empty artifacts | tmux |
| 45 | streaming | long response | no lost tail | tmux |
| 46 | streaming | response with newlines | line order is stable | tmux |
| 47 | streaming | repeated deltas | no duplicated content | unit |
| 48 | streaming | done without text | clean completion | unit |
| 49 | streaming | provider error after start | error closes turn | unit |
| 50 | streaming | network disconnect | error closes turn | tmux |
| 51 | lifecycle | cancel before response | request is aborted | tmux |
| 52 | lifecycle | cancel during thinking | no orphan activity | tmux |
| 53 | lifecycle | cancel then recover | next prompt works | tmux |
| 54 | lifecycle | queue two prompts | queue order is preserved | tmux |
| 55 | lifecycle | interject prompt | steering path is stable | tmux |
| 56 | lifecycle | rapid Enter | no duplicate submission | tmux |
| 57 | lifecycle | rapid typing | editor remains responsive | tmux |
| 58 | lifecycle | Esc clears prompt | editor clears only | tmux |
| 59 | lifecycle | Ctrl+C clears/aborts | correct state transition | tmux |
| 60 | lifecycle | quit during idle | clean process exit | tmux |
| 61 | lifecycle | quit during request | owned task shuts down | tmux |
| 62 | lifecycle | restart after quit | fresh session starts | tmux |
| 63 | lifecycle | provider error then retry | retry succeeds | tmux |
| 64 | lifecycle | repeated errors | no crash or task leak | tmux |
| 65 | lifecycle | session history | prior turns remain ordered | tmux |
| 66 | tools | tool-like prose | no accidental tool execution | tmux |
| 67 | tools | command request | capability policy is respected | tmux |
| 68 | tools | denied tool | error is rendered clearly | unit |
| 69 | tools | tool result text | output card is styled | unit |
| 70 | tools | tool output Markdown | tool output remains readable | unit |
| 71 | tools | tool failure | failure card is visible | unit |
| 72 | tools | tool cancellation | running card settles | unit |
| 73 | tools | duplicate call IDs | identities remain separate | unit |
| 74 | tools | long tool output | output truncation is safe | unit |
| 75 | tools | Unicode tool output | cell widths remain correct | unit |
| 76 | agents | sub-agent request | unsupported capability is explicit | tmux |
| 77 | agents | sub-agent setup event | activity card renders | unit |
| 78 | agents | sub-agent completion | completion card renders | unit |
| 79 | agents | sub-agent failure | failure is attributable | unit |
| 80 | agents | nested sub-agent | no cross-owner mutation | unit |
| 81 | agents | concurrent sub-agents | identities remain distinct | unit |
| 82 | agents | cancel sub-agent | owned task terminates | unit |
| 83 | agents | sub-agent long output | feed remains responsive | unit |
| 84 | skills | skill mention in prompt | provider treats it as text | tmux |
| 85 | skills | unsupported skill action | explicit limitation | tmux |
| 86 | skills | skill-like tool output | output is styled | unit |
| 87 | skills | multiple skill references | no state collision | unit |
| 88 | commands | `/help` | known unsupported command is typed error | tmux |
| 89 | commands | malformed known slash command | typed error, no prompt send | tmux |
| 90 | commands | unknown slash command | ordinary prompt policy | tmux |
| 91 | commands | `/clear` | clear behavior is explicit | tmux |
| 92 | commands | `/model` | model capability response is explicit | tmux |
| 93 | commands | slash command casing | classification is deterministic | unit |
| 94 | terminal | narrow terminal | no panic or corrupt layout | tmux |
| 95 | terminal | wide terminal | table and code fit | tmux |
| 96 | terminal | resize while idle | layout reflows | tmux |
| 97 | terminal | resize while streaming | layout remains stable | tmux |
| 98 | terminal | scroll history | navigation reaches older turns | tmux |
| 99 | terminal | copy selection | selected text is coherent | tmux |
| 100 | terminal | alternate screen exit | terminal is restored | tmux |
| 101 | security | secret absent from UI | key never renders | tmux |
| 102 | security | secret absent from error | errors redact credentials | unit |
| 103 | security | secret absent from git diff | no credential persisted | shell |
| 104 | resilience | HTTP 401 | actionable auth error | tmux |
| 105 | resilience | HTTP 429 | rate-limit message survives | unit |
| 106 | resilience | HTTP 500 | provider status survives | unit |
| 107 | resilience | timeout | request settles without orphan | tmux |
| 108 | resilience | response missing reasoning | text still renders | unit |
| 109 | resilience | response has reasoning | thinking renders once | unit |
| 110 | acceptance | full CI after live fixes | all repository gates green | shell |
