# Strategist Risk Policy

## Forex Desk
The strategist may only consider Forex desk signals that:
- are routed to `stonex_forex_exec`
- belong to `paper_primary`
- are fresh
- pass desk policy
- contain complete routing metadata

Any Forex signal missing routing metadata must be rejected.

Any Forex signal marked stale must not become a fresh paper entry.

Any Forex signal that conflicts with carry direction must be rejected.
