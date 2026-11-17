# Native attempt and association reports

Each independent case appears once in overall totals. Function and World views
reference that same structured case identity; they never add additional tests.

Each attempt retains its fresh process identity, scheduler/exclusion waiting time,
stdout/stderr paths, last published phase and primary/secondary diagnostics.
`callbacks` contains actual streamed source callback boundaries:

- Original file/byte extent and an attempt-local invocation ordinal.
- Actual lifecycle phase (1 example, 2 setup, 3 before-hook, 4 body, 5 after-hook,
  6 cleanup).
- Canonical World identity for setup/cleanup, not a source alias or address.
- Observed callback outcome and monotonic `duration_ms`.

Entry and completion events are flushed separately. Crashes/timeouts preserve
completed observations and mark an entered-but-unfinished boundary `interrupted`,
with no fabricated completed duration. Duplicate/reordered/mismatched boundaries
are infrastructure protocol errors. An incomplete callback cannot support a
worker's claim that the lifecycle completed.

By-function views show distinct associated cases, their outcomes/failure phases
and explicitly labeled **associated test duration**, not function execution time
or coverage. By-World views include per-case request origins and each attempt's
actual setup/cleanup observations. A null setup was not entered; null cleanup
means omitted or unentered, distinguished by `cleanup_declared`. Successful
cleanup is not proof that external resources were released.

Text, JSON and JUnit use the same canonical case/attempt records. JUnit retains
attempt details in each testcase's diagnostic payload. No source expression or
application operation is repeated to create any report. Coverage remains honestly
`not_collected`; timing is not used as a coverage approximation.
