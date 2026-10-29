# Reading `slice53/gates.log`

Every step in the log carries the exact command it ran and its own `EXIT=` line.

One step is **not** a test failure and must not be read as one:

`workspace_tests` in this script invocation exited 101 with no failing test.
`link.exe` exited `0xc0000142` (`STATUS_DLL_INIT_FAILED`) while building many
test binaries at once, after earlier interrupted runs on this host left orphaned
link processes behind. The log shows compile/link errors, not assertion
failures, and no `test result: FAILED` line exists in that step.

The authoritative workspace receipt at this head is two clean runs:

| file | result lines | passed | failed | ignored | exit |
| --- | --- | --- | --- | --- | --- |
| `workspace-tests.log` | 134 | 962 | 0 | 1 | 0 |
| `workspace-tests-rerun.log` | 134 | 962 | 0 | 1 | 0 |

The 134 result lines include 7 zero-test binaries and no doc-test headers
(`--all-targets` does not run doc tests). The previous slice's corresponding
all-targets receipt had 133 result lines / 947 passed; the +15 passed here are
14 initial Ruxa tests plus the printing-record case. These receipts precede the
recovery repair at `f7e1d41`; current focused/build/journey gates are in
`recovery-gates.log`, with 15 Ruxa cases after adding the graveyard-grant regression.

`stop-stale-cargo.py` exists because a killed background shell leaves `cargo`
children holding the target lock; it stops exactly those processes.
