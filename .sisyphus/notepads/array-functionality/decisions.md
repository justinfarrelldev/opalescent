# Decisions

- 2026-05-05T00:00:00Z Task 8 verification keeps array integration CLI invocations serialized around the shared `target/program` artifact instead of trying to parallelize them; this preserves the existing harness model while eliminating false negatives from binary overwrite races.
