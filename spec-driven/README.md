# spec-driven

Artifact container for the spec-driven-skills workflow. Each subdirectory is a feature identified by its slug.

## Structure

```
spec-driven/
  <slug>/
    spec.md              # Specification (requirements, acceptance criteria)
    plan.md              # Implementation plan (steps, architecture decisions)
    tasks.md             # Task decomposition (bundles, conflict analysis)
    bundle-N.md          # Individual bundle files (>15 tasks, or agent/team mode)
    progress.md          # Execution progress tracker
    team-instructions.md # Team mode coordination (when using --mode team)
  .sessions/             # Ephemeral session state (gitignored)
    <slug>.spec.json     # Spec elicitation session
    <slug>.plan.json     # Planning session
    <slug>.task.json     # Task decomposition session
```

## Workflow

Each feature flows: `spec -> design -> plan -> execute -> verify`

Artifacts are produced incrementally. A feature directory may contain only a `spec.md` if planning has not started yet.

## Session Files

The `.sessions/` directory contains ephemeral session state for resuming interrupted workflows. These files are gitignored and should not be committed. The `spec-driven/.sessions/` entry in `.gitignore` covers all session files with a single rule.
