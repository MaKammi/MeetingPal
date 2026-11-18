# Third-party source

`scap-0.1.0-beta.1` is the MIT-licensed crates.io release of CapSoftware/scap.
MeetingPal uses it only on Windows (Windows.Graphics.Capture) and macOS
(ScreenCaptureKit). Its Linux-only dependencies have been removed from the
normalized Cargo manifest to avoid two incompatible PipeWire native link
versions in Cargo's lock graph. Linux capture uses xcap with PipeWire 0.10.
The upstream license and original manifest are preserved. Two Windows release
compatibility defects are patched: the windows-capture 1.5 settings argument
order, and a call to the removed timespan method that also mixed 100 ns and QPC
units. MeetingPal uses its own shared monotonic video/audio timeline.

Target dimensions are re-exported for source selection. The macOS audio path fixes
explicit 48 kHz / stereo configuration and derives planar layout from the native
buffer count, so ScreenCaptureKit system audio can support macOS 14.2.
