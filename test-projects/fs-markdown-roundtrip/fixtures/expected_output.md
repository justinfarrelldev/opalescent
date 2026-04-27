# Opalescent Notes

> This fixture reads markdown from disk.
> It keeps the transform deterministic.

## Goals

> Read the committed input fixture.
> Turn paragraph lines into quotes.

### Details

> Heading lines stay unchanged.
> Blank lines stay blank.

## Checks

> Read the output back immediately.
> Compare bytes with the expected file.
> Keep reruns clean after tests.

### Extra

> No real markdown parser is used.
> The heuristic only depends on lines.

## Final

> Reruns should keep the repo clean.
> The integration test checks that.
