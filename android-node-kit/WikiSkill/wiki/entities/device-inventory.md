---
type: entity
updated: 2026-06-12
source_count: 1
tags: [devices, inventory, android]
---

# Device Inventory

Public template: `android-node-kit/inventory/nodes.example.csv`

Private local source: `android-node-kit/inventory/nodes.csv` (ignored by git)

The public inventory shows example rows only. Keep real devices in the ignored local `nodes.csv`.

| Name | Label | Host | SSH Port | User | Serial | Model |
| --- | --- | --- | --- | --- | --- | --- |
| old-tablet-lab-01 | Old Android tablet | 192.0.2.10 | 8022 | u0_a123 | REDACTED_SERIAL | REDACTED_MODEL |
| phone-worker-01 | Spare Android phone | 192.0.2.11 | 8022 | u0_a124 | REDACTED_SERIAL | REDACTED_MODEL |

## New Device Intake

When testing more edge devices over USB/ADB:

- Copy `android-node-kit/inventory/nodes.example.csv` to `android-node-kit/inventory/nodes.csv`.
- Add private device rows to the ignored local `nodes.csv`.
- Keep the `serial` value aligned with `adb devices` when possible.
- Fill `host`, `port`, and `user` after Termux SSH is bootstrapped.
- Record model from Android settings, `adb shell getprop ro.product.model`, or Termux node info.

## Links

- [[concepts/adb-usb-install]]
- [[syntheses/quant-m-edge-bundle-plan]]
