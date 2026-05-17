# Local Development

Masday Workflow should be understandable and runnable without Docker for the primary path.

## Default commands

```bash
pnpm install
pnpm build
cd apps/agent-runner && pnpm start:mcp
```

## Runtime profiles

By default, Masday runs in the **local** profile.

You can explicitly select a profile via environment variable:

```bash
export MASDAY_RUNTIME_PROFILE=local   # default
export MASDAY_RUNTIME_PROFILE=docker
export MASDAY_RUNTIME_PROFILE=remote
```

At the moment, only the **local** profile is implemented by the runtime. Selecting `docker` or `remote` will fail fast with a clear error.

Docker/remote are retained as optional/advanced documentation targets, but should not be assumed to work.



## Local-first expectations

- local development is the default workflow
- docs should not assume Docker is installed
- workflow/state terminology should match the canonical lifecycle
- advanced orchestration and intelligence features should be labeled by maturity, not implied to be universally active

## Docker and remote profiles

Docker is **optional**. Use it when you want:

- isolation from the host environment
- parity testing for containerized execution
- experimentation with alternate runtime packaging

Remote execution is an advanced/future-oriented direction and should be documented separately from the local happy path.

## Related docs

- [Getting started](../getting-started.md)
- [Workflow lifecycle](./lifecycle.md)
- [Architecture](../architecture.md)
