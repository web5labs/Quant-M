# Project Context Intake Question Bank

Use this bank selectively. Do not ask every question. Choose the smallest useful set for the current project state.

## When To Use

- The user has an idea but no written spec.
- The repo exists but the docs are thin or stale.
- The user has scattered notes and needs them turned into a usable build contract.
- The user is unsure what details matter.

## Core Questions

Start here when little context exists.

1. What is the project or product in one or two sentences?
2. Who is it for?
3. What problem does it solve?
4. What should a first shippable version be able to do?
5. Do you already have any files, notes, specs, screenshots, or references I should use?

## Existing Material Questions

Use when the user may already have source material.

1. Which of these already exists: README, PRD, spec, tickets, wireframes, API docs, transcripts, or repo notes?
2. Which file should be treated as the current source of truth?
3. Are any existing docs stale or partially wrong?
4. Are there example products, reference repos, or competitors we should study?

## Workflow Questions

Use when the main flow is unclear.

1. What is the core user journey from start to finish?
2. What is the single most important action the user must be able to complete?
3. What inputs does the user provide?
4. What output, result, or saved state should the system produce?
5. What should happen when things fail, are empty, or are still loading?

## Scope Questions

Use to reduce drift.

1. What is explicitly in scope for the first version?
2. What is explicitly out of scope?
3. Are there features that sound nice but should wait until later?
4. What would make you say "this is ready enough to ship"?

## Technical Questions

Use only when they matter to the next step.

1. Is there an existing stack, framework, or language requirement?
2. Does this need authentication, payments, AI features, external APIs, or background jobs?
3. Are there deployment constraints, hosting preferences, or cost limits?
4. Are there security, privacy, or compliance requirements?

## Team And Process Questions

Use when collaboration context matters.

1. Is this a new repo or an existing codebase?
2. Who will review or approve the spec?
3. Do you want a lightweight starter spec or a more complete build contract?
4. Should the final output be a quick brief, a full project spec, or both?

## Specless Fallback

If the user has no spec at all, guide them toward answering enough to fill:

- `docs/project-definition.md`
- `docs/project-spec.md`
- `docs/open-questions.md`
- `docs/assumptions.md`

Minimum fields to resolve:

- project summary,
- target user,
- core problem,
- MVP outcome,
- core workflow,
- functional requirements,
- non-goals,
- constraints,
- validation idea,
- open questions.
