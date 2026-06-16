---
name: project-context-intake
description: Gather project context before onboarding or implementation by asking the user a structured series of questions, requesting existing project spec files or source documents, identifying missing context, and helping draft a project spec when one does not exist. Use when the user has an idea, partial repo, scattered notes, or unclear requirements and needs help turning them into actionable project context.
---

# Project Context Intake Skill

Use this skill before onboarding or implementation when the project context is incomplete, scattered, or missing a usable spec.

Your job is to gather only the context needed to make the next step clear:

- what the project is,
- what files or specs already exist,
- what the user wants built now,
- what constraints matter,
- and whether a real project spec must be created.

## First Move

1. Inspect the repo for obvious context first.
2. Read existing files before asking broad questions.
3. Ask only for missing information.
4. If the user already has a spec, ask them to attach it or point to the file path.
5. If the user does not have a spec, switch into guided spec creation.

## Interview Rules

- Ask questions in small batches, usually 3 to 6 at a time.
- Prefer concrete prompts over vague brainstorming.
- Prefer either/or or short-answer questions when possible.
- If the user provides files, summarize them before asking the next batch.
- Do not ask questions whose answers are already in the repo.
- Do not overwhelm the user with a giant questionnaire in one message.
- If an answer is uncertain, record it as an assumption or open question instead of pretending it is settled.

## Intake Flow

### 1. Check for existing context

Look for:

- `README*`
- `AGENTS.md`
- `docs/`
- `docs/project-spec.md`
- `docs/project-definition.md`
- PRDs, RFCs, notes, transcripts, exported tickets, or planning docs

If any of these exist, ask the user to confirm which ones are current and which ones are obsolete.

### 2. Determine spec status

Ask one direct question early:

- "Do you already have a project spec, PRD, README, or notes I should use?"

If yes:

- ask for file paths, pasted text, or attachments,
- read them first,
- summarize what they say,
- ask only the questions needed to fill the remaining gaps.

If no:

- tell the user you can help create a spec from scratch,
- move into guided discovery using the question bank in `references/intake-question-bank.md`.

### 3. Gather the minimum viable context

Cover these areas before moving on:

- product or project summary,
- target user,
- core problem,
- main outcome,
- must-have workflow,
- constraints or non-goals,
- stack or integration constraints,
- validation or success criteria,
- timeline or urgency if relevant.

### 4. Create or refine the spec

If a usable spec is missing, draft one using the user's answers and existing repo evidence.

Default outputs:

- `docs/codex/intake-summary.md`
- `docs/project-definition.md`
- `docs/project-spec.md`
- `docs/open-questions.md`
- `docs/assumptions.md`

When drafting:

- mark assumptions clearly,
- separate confirmed facts from inferred details,
- list unresolved decisions,
- keep the first draft practical and build-oriented.

### 5. Handoff to onboarding

Once the context is sufficient, recommend the next step:

- run `model-project-onboarding` if the project foreground still needs to be scaffolded,
- or proceed with implementation only if onboarding/spec readiness is already satisfied.

## Output Contract

At the end of the intake, provide:

```text
Project summary:
Existing context files:
Spec status:
Confirmed requirements:
Assumptions:
Open questions:
Recommended files to create or update:
Recommended next step:
```

## Reference

Read `references/intake-question-bank.md` when the user has no spec, has only vague notes, or needs help clarifying scope.
