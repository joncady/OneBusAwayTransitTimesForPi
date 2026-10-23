# PM3 touchscreen follow-up — 2026-10-22

**Archived / not planned:** the owner chose transit cards and QR codes as the
PM3 interface. No further touchscreen investigation or integration is planned.
Issue #12 is closed as not planned; the findings below are historical.

## Result

No working touchscreen is established on this unit. This follow-up reviewed the
saved inventory and hardware documentation while the device was off. It did not
contact the device, attempt ignition, reboot it, change settings or capture taps.
Tracked follow-up: [issue #12](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/12).

## Evidence and limits

- The RIPTA system design document's page 9 block diagram shows both the touch
  panel and its controller as optional dashed boxes. The controller connects to
  an internal USB hub; the LCD has a separate 24-bit RGB connection. A working
  LCD does not establish that the optional touch hardware is fitted.
- The saved USB inventory contains the root hubs, internal hub/media reader,
  barcode UART converter and USB stick, with no identified touch controller.
  No usable touch input device was exposed during the audit. This does not
  distinguish absent hardware from a disconnected or unpowered controller.
- Kernel settings include `CONFIG_HID_MULTITOUCH=y` and `CONFIG_USB_HID=y`, while
  `CONFIG_INPUT_TOUCHSCREEN`, `CONFIG_TOUCHSCREEN_FTS` and `CONFIG_I2C_HID` are
  disabled. A compatible USB HID touch device may use the enabled HID path;
  disabled dedicated touch drivers alone do not prove USB touch is impossible.
- The stock UI contains `TouchPanel=true` and a tslib `/dev/input/event0`
  configuration. These are software configuration, not proof of a fitted
  controller or a usable event device. Installed tslib plugins are likewise
  not evidence of physical hardware.
- No named touch controller appears in the saved device-tree strings. USB
  devices can enumerate without their own device-tree node, so this is not
  conclusive evidence of absence.
- The native Rust/Slint app currently renders directly to the framebuffer and
  processes NFC/QR input; it does not read evdev touch events or dispatch Slint
  pointer events. Hardware detection and application input integration are
  separate remaining steps.

## References

- [INIT RIPTA system design document](https://kevin.wallace.seattle.wa.us/pm3/ripta-system-design-document.pdf), page 9, optional panel/controller and USB path. Product-variant documentation, not proof of this unit's fitted components.
- [PM3 main-board notes](https://wiki.pm3.dev/main_board_overview), internal USB hub and barcode converter identification.
- [Initial hardware/software audit](pm3-audit-2026-10-21.md), observed input/kernel state.
