---
title: "Quant-M - Quant-M Adversarial Review"
source: "https://chatgpt.com/g/g-p-69c1fcc211fc819193bfaee6562f7533-quant-m/c/6a2b2790-9cc4-832c-8747-610eaae6446b"
author:
published:
created: 2026-06-14
description: "ChatGPT is your AI chatbot for everyday use. Chat with the most advanced AI to explore ideas, solve problems, and learn faster."
tags:
  - "clippings"
---
## Adversarial Review: Quant-M (June 2026)

Based on the recent Quant-M discussions, the runtime status updates, governance packets, fuzz testing, channel additions, shared-state architecture proposals, and the recent Telegram/chat channel work, I would currently score Quant-M:

| Category | Score |
| --- | --- |
| Architecture | 9.0/10 |
| Safety | 9.5/10 |
| Extensibility | 9.5/10 |
| Multi-Agent Design | 9.0/10 |
| Operator Experience | 7.0/10 |
| Onboarding | 5.5/10 |
| Market Readiness | 6.5/10 |
| Trading Readiness | 5.0/10 |
| Research Runtime | 8.5/10 |
| Community Adoption Potential | 8.0/10 |

**Overall: 8.1/10 as an architecture project.**

**Overall: 6.3/10 as a product someone would download today.**

---

## What Quant-M Gets Right

Most agent frameworks start with:

> "Let's build agents."

Quant-M starts with:

> "Let's build governance."

That is a major difference.

Recent work has focused heavily on:

- replayability
- evidence tracking
- FSM execution
- approval gates
- policy enforcement
- cost tracking
- context budgets
- compressed handoffs
- operator dashboards

This is much closer to how regulated systems are built than typical AI agent projects.

Recent additions such as:

- channel management
- Telegram support
- chat adapters
- runtime evidence previews
- import previews
- operator dashboards
- reconciliation reports

move Quant-M further toward becoming an execution operating system rather than another chatbot wrapper.

---

## Major Strength #1

## FSM First

Most agent frameworks:

```markdown
LLM
 ↓
 Decision
 ↓
 Action
```

Quant-M:

```markdown
LLM
 ↓
 Evidence
 ↓
 Shared State
 ↓
 FSM
 ↓
 Policy Gate
 ↓
 Action
```

That is a far safer architecture.

The LLM becomes:

- analyst
- researcher
- planner

rather than:

- trader
- executor
- administrator

This is the correct separation.

---

## Major Strength #2

## Shared State Direction

The recent LLM Wiki concept is one of the strongest architectural improvements discussed recently.[Quant M architecture proposal](https://chatgpt.com/g/g-p-69c1fcc211fc819193bfaee6562f7533/c/69e4cbb5-b6a8-83ea-adf9-192ba4ef70cb?messageId=7c35474e-a7cd-4366-9df7-9e73544255b5&conversation_save=1)

Instead of:

```markdown
Agent A
Agent B
Agent C

all rediscovering facts
```

You are moving toward:

```markdown
Raw Data
 ↓
 Distillation
 ↓
 Wiki Layer
 ↓
 Weighted State
 ↓
 FSM
```

That is potentially a huge efficiency gain.

Especially for:

- Forex
- Crypto
- Sports
- Macro

desks.

---

## Major Strength #3

## Multi-Model Orchestration

This remains Quant-M's biggest differentiator.

Most systems:

```markdown
One model
One context
One answer
```

Quant-M vision:

```markdown
Gemini
Claude
OpenAI
Grok
DeepSeek
OpenRouter

running concurrently

producing weighted evidence
```

Very few frameworks currently support this concept well.

Even fewer support it while keeping execution deterministic.

---

## Major Strength #4

## Recent Channel System

The new channel work was the correct move.

The ability to separate:

```markdown
Research Channels
Execution Channels
Operator Channels
Chat Channels
Telegram Channels
```

is much more scalable than treating everything as one conversation.

Benefits:

### Telegram

Potential uses:

- approvals
- alerts
- FSM notifications
- status updates
- incident reports

without exposing the runtime itself.

### Chat Channels

Potential uses:

- staff communication
- lane communication
- human approvals
- escalation requests

without polluting execution logs.

This is very similar to how real operations teams separate:

- Slack
- PagerDuty
- Jira
- CI/CD

rather than combining everything.

The recent channel work was a strong architectural decision.

---

## Major Weakness #1

## Nobody Knows What Quant-M Is

This remains the largest problem.

Ask ten people:

> What is Quant-M?

You will likely get:

1. Trading system
2. Agent framework
3. Research tool
4. FSM runtime
5. Multi-agent platform
6. Quant finance platform
7. Trading copilot
8. Orchestration layer

This is a branding problem.

The architecture is becoming clearer than the product.

---

## Major Weakness #2

## Onboarding Is Still Too Technical

Current onboarding requires understanding:

- providers
- models
- APIs
- channels
- FSMs
- policies
- contexts
- lanes

Most users won't survive that process.

Compare that to:

### Claude Code

```markdown
Install
Run
```

### Codex

```markdown
Install
Run
```

### OpenClaw

```markdown
Install
Configure
Run
```

Quant-M currently feels closer to:

```markdown
Install

Read docs

Configure providers

Configure channels

Configure FSMs

Configure policies

Maybe run
```

This is why your recent onboarding wizard concept is extremely important.

---

## Major Weakness #3

## No Signature Experience Yet

Claude Code has:

> coding

Hermes has:

> coding

OpenClaw has:

> autonomous coding

Quant-M currently has:

> architecture

Architecture is valuable.

But users buy experiences.

Not architectures.

---

## Major Weakness #4

## Context Explosion Risk

The more desks you add:

- Sports
- Crypto
- Forex
- Macro
- Coding
- Research

the more likely shared state becomes:

```markdown
global junk drawer
```

Recent compressed handoff work helps.

Context budgets help.

But this remains a serious risk.

---

## Major Weakness #5

## Telegram Could Become a Liability

Telegram is powerful.

But it creates attack surfaces.

Potential issues:

### Notification Storms

50 agents

↓

500 notifications

↓

operator ignores everything

### Approval Spam

Too many approvals.

Operator fatigue.

### Token Leakage

If not isolated properly:

- prompts
- evidence
- API metadata

could leak into channels.

The Telegram layer should remain:

```markdown
Notification Plane
```

not

```markdown
Execution Plane
```

---

## Major Weakness #6

## Too Much Governance Too Early

This is controversial.

The governance work is excellent.

However:

You now have:

- dashboards
- previews
- import previews
- evidence previews
- reconciliation reports
- adversarial reviews
- context budgets
- packet plans

before having a killer use case.

There is a danger of:

```markdown
building management
before workers exist
```

---

## Largest Technical Risk

## Shared State Becomes a Database Instead of Intelligence

The recent LLM Wiki proposal is excellent.[Quant M architecture proposal](https://chatgpt.com/g/g-p-69c1fcc211fc819193bfaee6562f7533/c/69e4cbb5-b6a8-83ea-adf9-192ba4ef70cb?messageId=7c35474e-a7cd-4366-9df7-9e73544255b5&conversation_save=1)

But many projects fail here.

Shared state becomes:

```markdown
huge storage layer
```

instead of:

```markdown
high signal memory layer
```

You need:

- decay
- contradiction tracking
- confidence
- source lineage
- weighted scoring

or the system will drown in its own memory.

The recent proposal correctly identifies this risk.[Quant M architecture proposal](https://chatgpt.com/g/g-p-69c1fcc211fc819193bfaee6562f7533/c/69e4cbb5-b6a8-83ea-adf9-192ba4ef70cb?messageId=7c35474e-a7cd-4366-9df7-9e73544255b5&conversation_save=1)

---

## What Would Break Quant-M First?

If launched tomorrow:

### 1

Onboarding complexity.

### 2

Provider setup confusion.

### 3

Lack of obvious use case.

### 4

Too many knobs.

### 5

Operator overload.

Not the runtime.

Not Rust.

Not FSMs.

Not channels.

The human experience.

---

## What Shippable Looks Like

For a v0.1 public release I would narrow Quant-M to:

### Quant-M Research Runtime

Features:

✅ Multi-model orchestration

✅ Shared state

✅ Evidence system

✅ Telegram alerts

✅ Chat channels

✅ FSM approvals

✅ Cost tracking

✅ Replay

Ignore:

❌ Live trading

❌ Full desk ecosystem

❌ Complex workflow builders

❌ Autonomous execution

❌ Massive plugin systems

---

## Current Verdict

Quant-M is no longer just an agent framework.

It is becoming:

> A governed multi-model orchestration runtime with evidence tracking, shared state intelligence, policy enforcement, channelized communication, and deterministic FSM execution.

The recent channel work and Telegram integration improve operator usability significantly.

The recent shared-state and LLM Wiki direction may be the single most important optimization discussed so far because it gives Quant-M something many agent frameworks lack:

> persistent, weighted institutional memory instead of temporary context windows.> 
> [Quant M architecture proposal](https://chatgpt.com/g/g-p-69c1fcc211fc819193bfaee6562f7533/c/69e4cbb5-b6a8-83ea-adf9-192ba4ef70cb?messageId=7c35474e-a7cd-4366-9df7-9e73544255b5&conversation_save=1)

The biggest remaining challenge is no longer the architecture.

It's turning that architecture into a product that someone can install, understand, and successfully use in under 10 minutes.

## Quant-M Adversarial Hardened Roadmap

## Goal: Move from "Excellent Architecture" → "Unquestionably Shippable"

The purpose of an adversarial roadmap is not to add features.

The purpose is to eliminate the reasons Quant-M could fail.

---

## Phase 0 — Define the Battlefield

Before writing more code:

Answer one question.

## What is Quant-M?

Current answer:

```markdown
Multi-model
FSM
Trading
Research
Governance
Agent Runtime
Shared State
Execution Engine
```

This is too broad.

### Hardened Position

Quant-M is:

> A governed multi-model orchestration runtime that transforms evidence into deterministic actions through FSMs.

Everything else becomes a module.

Not the product.

---

## Phase 1 — Attack the Biggest Risk

## Problem

Nobody can install it.

The architecture is becoming stronger than the onboarding.

---

## Build Quant-M First Run Wizard

When users launch:

```markdown
quantm init
```

They should see:

```markdown
Welcome to Quant-M

What are you trying to do?

1. Coding
2. Research
3. Trading Research
4. Multi-Agent Workspace
```

---

### Step 1

Choose providers.

Example:

```markdown
Detected:

✓ OpenAI Codex
✓ Claude Code
✓ Gemini CLI

Use detected providers?

[Y/N]
```

---

### Step 2

Select Runtime Mode

```markdown
Research Mode
Development Mode
Execution Mode
```

---

### Step 3

Create Workspace

Quant-M generates:

```markdown
quantm.toml

/workspace
/shared_state
/evidence
/replay
/channels
```

automatically.

---

### Success Metric

Installation to usable state:

Current estimate:

```markdown
30-60 minutes
```

Target:

```markdown
5 minutes
```

---

## Phase 2 — Harden Shared State

Recent conversations revealed this is likely the future heart of Quant-M.

The danger:

```markdown
Shared State
=
Data Dump
```

instead of

```markdown
Shared State
=
Institutional Memory
```

---

## Build Knowledge Decay Engine

Every entry receives:

```markdown
confidence
reliability
freshness
decay_rate
source_count
contradiction_count
```

---

### Memory Classes

Ephemeral

```markdown
minutes → days
```

Tactical

```markdown
days → weeks
```

Strategic

```markdown
months
```

Canonical

```markdown
years
```

This idea aligns with the recent LLM Wiki direction discussed for Quant-M.[Quant M architecture proposal](https://chatgpt.com/g/g-p-69c1fcc211fc819193bfaee6562f7533/c/69e4cbb5-b6a8-83ea-adf9-192ba4ef70cb?messageId=7c35474e-a7cd-4366-9df7-9e73544255b5&conversation_save=1)

---

## Phase 3 — Harden Channels

Recent Telegram and chat channel work is excellent.

But it creates new risks.

---

## Rule 1

Channels cannot execute.

Channels can only:

```markdown
observe
approve
deny
escalate
```

---

## Rule 2

Telegram is Notification Only

Never:

```markdown
telegram -> execute trade
```

Always:

```markdown
telegram -> approval request
```

then

```markdown
operator approval
```

then

```markdown
FSM evaluation
```

---

## Rule 3

Channel Isolation

Research channels:

```markdown
read only
```

Execution channels:

```markdown
restricted
```

Operator channels:

```markdown
approval only
```

---

## Phase 4 — Harden Multi-Agent Operations

Current danger:

Agent loops.

Example:

```markdown
Agent A
asks B

Agent B
asks C

Agent C
asks A
```

Infinite loop.

---

## Build Agent Budget System

Each task receives:

```markdown
max_iterations
max_cost
max_time
max_context
```

---

### Automatic Kill Conditions

If exceeded:

```markdown
STOP
ESCALATE
```

not

```markdown
CONTINUE
```

---

## Phase 5 — Harden Provider Layer

Current future vision:

```markdown
OpenAI
Anthropic
Gemini
Grok
DeepSeek
OpenRouter
```

Excellent.

But fragile.

---

## Create Provider Normalization Layer

Every provider becomes:

```markdown
ModelRequest
ModelResponse
ToolCall
CostRecord
```

No provider-specific logic above this layer.

---

### Goal

Switch:

```markdown
Claude
```

to

```markdown
Gemini
```

without changing workflows.

---

## Phase 6 — Harden Evidence

Currently one of Quant-M's strongest ideas.

---

## Every Action Must Answer

Why?

What evidence?

Which policy?

Which model?

Which source?

Which version?

---

### Replay Requirements

Every execution produces:

```markdown
Evidence Package
```

containing:

```markdown
Inputs
Models
Outputs
FSM State
Policies
Result
```

---

### Success Test

Can you replay six months later?

If not:

Not hardened.

---

## Phase 7 — Harden Cost Control

Future enterprise users will care more about cost than intelligence.

---

## Add Cost Governor

Before model invocation:

```markdown
estimated_cost
```

After invocation:

```markdown
actual_cost
```

Track:

```markdown
per agent
per workflow
per desk
per day
```

---

### Emergency Controls

```markdown
Daily Budget

Soft Limit
Hard Limit
Kill Limit
```

---

## Phase 8 — Harden Trading Path

This is the most dangerous area.

---

## Current Recommendation

Remain:

```markdown
Research Only
```

for public releases.

---

## Required Before Live Execution

### Market State Validation

Exchange reachable?

Spread healthy?

Latency healthy?

Market open?

---

### Risk State Validation

Max drawdown exceeded?

Exposure exceeded?

Position conflict?

---

### Policy Validation

Operator approval?

Desk approval?

Risk approval?

---

Only then:

```markdown
FSM
```

may execute.

---

## Phase 9 — Harden Operator Experience

Current architecture score:

```markdown
9/10
```

Operator experience:

```markdown
7/10
```

---

## Build Quant-M Control Center

Single pane:

```markdown
Agents
Channels
Evidence
Shared State
Costs
Approvals
Alerts
```

---

Not:

```markdown
12 terminals
```

for everyday users.

Keep terminals for power users.

---

## Phase 10 — Quant-M Signature Feature

This is the most important phase.

---

## The Hard Question

Why use Quant-M instead of:

- Claude Code
- OpenClaw
- Hermes
- Codex
- Empire LLM

---

### Proposed Answer

**Evidence-Oriented Multi-Model Consensus**

Example:

```markdown
Gemini
Claude
OpenAI
DeepSeek
```

all evaluate:

```markdown
research question
```

↓

Shared State

↓

Weighted Scoring

↓

Consensus Score

↓

FSM

↓

Operator Approval

↓

Action

```markdown
That workflow is currently rare.

It is also difficult to copy.

---

# Definition of Hardened v1.0

Quant-M becomes hardened when:

### Reliability

- Crash recovery works
- Replay works
- Journaling works

### Governance

- Policies enforced
- Approvals enforced
- Evidence attached

### Shared State

- Decay implemented
- Contradiction tracking implemented
- Confidence scoring implemented

### Channels

- Telegram isolated
- Chat isolated
- No direct execution

### Cost

- Budget controls active
- Per-agent accounting active

### User Experience

- Installation < 5 minutes
- First workflow < 10 minutes

### Product Identity

A user can explain Quant-M in one sentence:

> "Quant-M is a governed multi-model orchestration runtime that turns evidence into deterministic actions."

Once those milestones are complete, the biggest risks shift away from architecture and toward adoption, community growth, and ecosystem expansion.
```

Absolutely. For TTS, the roadmap is much easier to understand if it's explained like a project manager briefing rather than a technical document.

---

Quant-M is at a very interesting stage right now.

The architecture is becoming stronger than most agent frameworks, but the biggest risks are no longer technical. The biggest risks are onboarding, usability, operator overload, and product clarity.

The purpose of the adversarial roadmap is simple:

Imagine Quant-M becomes successful tomorrow.

What are the first things that would break?

Then fix those before they happen.

---

Phase One is onboarding.

Right now, Quant-M makes sense to people like you because you've spent months discussing FSMs, shared state, agent orchestration, governance, and multi-model systems.

A new user has not.

A new user wants to install Quant-M and get value within five minutes.

The long-term goal should be something like:

"Install Quant-M."

"Choose what you want to do."

"Connect your preferred providers."

"Start working."

No manuals.

No giant configuration files.

No reading documentation for an hour.

The onboarding wizard may end up being one of the most important features in the entire project.

---

Phase Two is shared state.

This is probably the most important architectural direction discussed recently.

The danger is that shared state becomes a giant database filled with information nobody can effectively use.

Instead, shared state should become institutional memory.

Think of it like hiring an employee who never forgets anything important, but also knows when old information is no longer relevant.

Recent information should have more weight.

Contradicted information should lose trust.

Frequently confirmed information should gain authority.

Over time, Quant-M should become smarter because its memory improves, not because its context window gets larger.

This is where the LLM Wiki concept becomes powerful.

---

Phase Three is channel hardening.

The recent Telegram and chat channel work is a very good addition.

However, communication systems can become dangerous if they are not properly isolated.

Telegram should become the notification layer.

Not the execution layer.

A Telegram message should be able to tell you:

"An event occurred."

"Approval is required."

"An agent completed a task."

But a Telegram message should never directly trigger execution.

The FSM should always remain in control.

Think of Telegram as a pager, not a steering wheel.

---

Phase Four is preventing agent loops.

This is a problem many agent frameworks eventually encounter.

One agent asks another agent a question.

That agent asks a third agent.

The third agent asks the first agent.

Suddenly you have three agents talking endlessly and consuming money.

Every agent should have limits.

Limits on time.

Limits on cost.

Limits on iterations.

Limits on context usage.

When those limits are reached, Quant-M should stop automatically and ask for operator guidance.

---

Phase Five is provider independence.

One of Quant-M's biggest strengths is support for many providers.

OpenAI.

Anthropic.

Gemini.

DeepSeek.

Grok.

OpenRouter.

Future providers.

The danger is becoming dependent on any one provider.

The solution is creating a normalization layer.

To Quant-M, all providers should look identical.

That means if a provider changes pricing, policies, or availability, Quant-M continues functioning with minimal disruption.

---

Phase Six is evidence.

This is one of Quant-M's strongest features today.

Every action should answer six questions.

What happened?

Why did it happen?

What evidence supported it?

Which model participated?

Which policy allowed it?

What result occurred?

This creates accountability.

It also creates replayability.

Months later, an operator should be able to replay the entire workflow and understand exactly why a decision was made.

Most agent frameworks cannot do this.

Quant-M can.

---

Phase Seven is cost governance.

As models become more powerful, they also become more expensive.

Future users may care more about cost control than model intelligence.

Quant-M should become extremely good at cost tracking.

Every workflow.

Every agent.

Every desk.

Every provider.

Every model.

Should have cost visibility.

The dream is that Quant-M eventually becomes smarter about spending than the operator.

It should know when a task requires Claude.

When it requires Gemini.

When a cheap model is sufficient.

And when no model is needed at all.

---

Phase Eight is trading safety.

This is where many AI projects fail.

The temptation is to move toward autonomous execution too quickly.

My recommendation remains the same.

Keep public Quant-M focused on research first.

Execution later.

The research engine itself has enormous value.

The day Quant-M touches real money, everything changes.

Now you need risk controls.

Exposure limits.

Market validation.

Approval systems.

Circuit breakers.

Audit trails.

That should happen after the research foundation is mature.

---

Phase Nine is operator experience.

Right now Quant-M is becoming very powerful.

But powerful does not always mean easy to use.

You don't want users managing twelve terminal windows forever.

Power users will love terminals.

Most users won't.

Eventually Quant-M needs a command center.

One place where users can see:

What agents are running.

What tasks are active.

What approvals are waiting.

What costs have accumulated.

What evidence has been generated.

And what actions are being recommended.

The architecture already exists.

Now it needs a cockpit.

---

Phase Ten is the most important phase.

Finding Quant-M's signature feature.

The question every successful product answers is:

"Why should someone choose this instead of everything else?"

Claude Code already owns coding.

OpenClaw owns autonomous coding workflows.

Hermes focuses on orchestration.

Codex focuses on OpenAI-native development.

Quant-M's strongest answer may be something very different.

Evidence-driven multi-model consensus.

Instead of asking one model for an answer, Quant-M asks several models.

It compares their findings.

Weights their evidence.

Stores the results in shared state.

Runs that information through an FSM.

And only then produces a recommendation or action.

That is a much more disciplined process than simply asking a single model for an opinion.

---

If I had to summarize the entire roadmap in one sentence:

Quant-M's next challenge is no longer building a stronger architecture.

It is transforming a powerful architecture into a product that ordinary users can install, understand, trust, and use successfully within their first ten minutes.

For Quant-M, I would redefine **shippable** because most software projects use the wrong definition.

Most people define shippable as:

> "The code compiles and doesn't crash."

That is not enough for Quant-M.

Quant-M is not a simple application.

It is a governed orchestration runtime.

That means the definition of shippable must be much higher.

---

## Quant-M Definition of Shippable

A feature is shippable when:

> A new user can successfully use it, understand it, recover from failure, and trust the results without requiring the developer.

That means five things must be true.

---

## 1\. It Works

The obvious requirement.

The feature functions correctly.

No crashes.

No broken commands.

No corrupted state.

No critical bugs.

This is the minimum requirement.

Not the final requirement.

---

## 2\. It Is Understandable

If a user cannot explain what happened after using the feature, it is not shippable.

Example:

Bad:

> "The orchestration subsystem completed a reconciliation event."

Good:

> "Three models reviewed the task. Two agreed. One disagreed. The recommendation was approved."

The operator should understand what happened.

---

## 3\. It Is Recoverable

Every failure must have a recovery path.

Example:

Bad:

> Agent crashed.

Good:

> Agent crashed.
> 
> Resume from checkpoint?
> 
> View evidence?
> 
> Replay session?

If failure requires Julio to manually fix something, it is not shippable.

---

## 4\. It Is Observable

The operator must be able to answer:

What is running?

What completed?

What failed?

What is waiting?

What did it cost?

Why did it happen?

If the system cannot explain itself, it is not shippable.

---

## 5\. It Is Governed

This is where Quant-M differs from most agent frameworks.

Every action must have:

Evidence.

Policy.

Approval path.

Audit trail.

Replay capability.

A result without evidence is not shippable.

---

## Quant-M Levels of Shippable

I would actually define four levels.

---

## Level 1 — Functional

The code works.

Example:

A Telegram notification sends successfully.

Most open-source projects stop here.

---

## Level 2 — Usable

A new user can figure it out.

No documentation required.

No developer assistance required.

This is where most commercial software lives.

---

## Level 3 — Governed

The action is traceable.

Evidence exists.

Approvals exist.

Replay exists.

Audit exists.

This should be Quant-M's minimum target.

---

## Level 4 — Operational

This is the gold standard.

The system continues working under:

Failures.

Provider outages.

Agent crashes.

Bad model responses.

Context exhaustion.

Network interruptions.

This is where enterprise systems live.

---

## Quant-M v1 Definition of Shippable

I would formally define it as:

> A capability is shippable when it can be installed, operated, audited, recovered, and trusted by a non-developer without assistance.

---

## Quant-M Definition of "Done"

Many projects confuse "done" with "merged."

For Quant-M:

Done is not:

- Code written
- Tests passing
- Pull request merged

Done is:

- Works correctly
- Survives failure
- Has evidence
- Has replay
- Has documentation
- Has operator visibility
- Fits onboarding flow
- Fits governance flow

Only then is it done.

---

## Quant-M Release Gate

Before any feature is declared shippable, ask these 10 questions:

1. Does it work?
2. Can a new user understand it?
3. Can it recover from failure?
4. Is it observable?
5. Does it generate evidence?
6. Can it be replayed?
7. Does it respect policy?
8. Does it respect cost controls?
9. Does it fit onboarding?
10. Would I trust it at 3 AM when nobody is available to fix it?

If the answer to any of those is "no," it is not shippable yet.

---

### TTS-Friendly One-Sentence Version

**For Quant-M, shippable means a feature works, explains itself, survives failure, produces evidence, can be replayed, and can be trusted by a user without needing the developer to help them.**