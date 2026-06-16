# PONboarding To Staff OS Handoff

## Purpose

This contract is the durable handoff from project onboarding into execution.

PONboarding owns:

- project framing
- accepted and rejected references
- documentation map
- project spec and shippable scaffolds
- repo research findings
- model stack recommendations
- recommended API stack with homepage, docs, and API-key links
- delegation notes

Staff OS consumes that output and turns it into:

- runtime routing
- staffing assignments
- task execution
- approvals
- audit trails
- memory writeback

## Canonical schema

Primary schema:

- `docs/contracts/ponboarding-handoff.schema.json`

Supporting schemas:

- `docs/contracts/repo-scorecard.schema.json`
- `docs/contracts/model-stack-recommendation.schema.json`

## Required top-level sections

- `contractVersion`
- `generatedAt`
- `sourceProject`
- `projectContext`
- `repoResearch`
- `recommendedApiStack`
- `modelStack`
- `delegationPlan`
- `staffOsIntake`

## Handoff rules

- PONboarding should emit scaffolds and bounded recommendations, not pretend final project details are settled when they are still deferred.
- The contract should preserve uncertainty explicitly through `openQuestions`, `assumptions`, `status`, and decision notes.
- Repo research must keep accepted, rejected, and watchlist references separate.
- Model recommendations must explain why a model is proposed for a lane and which fallback models are acceptable.
- Recommended providers, tools, and support resources should include homepage links and API-key or console links whenever they exist.
- Staff OS should treat this handoff as planning input plus memory input, not as permission to skip validation.

## Minimal delivery expectation

At minimum, a handoff should contain:

- one-line project thesis
- outcome and success criteria
- spec and shippable status
- accepted reference set
- repo scorecards for researched candidates
- model lane recommendations
- recommended provider and tool API surfaces with link metadata
- delegation plan
- selected tools, providers, and channels for initial Staff OS intake
