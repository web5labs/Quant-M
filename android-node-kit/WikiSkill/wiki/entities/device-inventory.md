---
type: entity
updated: 2026-06-12
source_count: 1
tags: [devices, inventory, android]
---

# Device Inventory

Source: `android-node-kit/inventory/nodes.csv`

The current inventory records 5 Android edge devices:

| Name | Label | Host | SSH Port | User | Serial | Model |
| --- | --- | --- | --- | --- | --- | --- |
| old-tablet | Old tablet | 10.0.0.10 | 8022 | u0_a170 | 7XS88LMJPN5TONUG | 9032Z |
| zte-android | Zte android | 10.0.0.248 | 8022 | u0_a111 | 6effee7f | Z831 |
| umx-android | Umx android | 10.0.0.32 | 8022 | u0_a91 | 5551036 | U683CL |
| vortex-tablet | Vortex Tablet | 10.0.0.47 | 8022 | u0_a189 | T10MPRO00281557 | T10M_Pro |
| dialN | dialN | 10.0.0.16 | 8022 | u0_a183 | 8X622404007690 | X62 |

## New Device Intake

When testing more edge devices over USB/ADB:

- Add a row to `android-node-kit/inventory/nodes.csv`.
- Keep the `serial` value aligned with `adb devices` when possible.
- Fill `host`, `port`, and `user` after Termux SSH is bootstrapped.
- Record model from Android settings, `adb shell getprop ro.product.model`, or Termux node info.

## Links

- [[concepts/adb-usb-install]]
- [[syntheses/quant-m-edge-bundle-plan]]
