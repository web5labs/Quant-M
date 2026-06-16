# UI/UX Handoff

## Functional flow summary

Quant-M currently exposes a CLI/runtime workflow rather than a browser UI. Any future UX handoff should start from the command flows documented in `README.md` and the runtime states documented in the onboarding spec.

## Screens/routes implemented

None required in the current phase.

## Components implemented

No UI components are in scope for the current functional runtime.

## Current UX weaknesses

Configuration and operational flows are CLI- and file-driven, which may become harder to onboard as the runtime surface grows.

## Human decisions needed

Decide whether Quant-M should remain CLI-first indefinitely or eventually gain a small operator surface after the core runtime is stable.

## Suggested design direction

If a UI is ever approved, keep it narrow: status, queue visibility, heartbeat visibility, and safe configuration review before any execution controls.

## Accessibility concerns

Not applicable yet because no browser UI is in scope.

## Responsive layout concerns

Not applicable yet because no browser UI is in scope.

## Copy/messaging concerns

Future UI copy should preserve the product's local-first, safe-by-default posture rather than sounding like a general autonomous agent platform.

## Visual polish backlog

Deferred until after the core runtime reaches a stable functional review state.
