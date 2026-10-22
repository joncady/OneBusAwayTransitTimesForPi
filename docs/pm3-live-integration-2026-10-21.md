# PM3 live arrivals integration — 2026-10-21

## Implemented and deployed

- One native Rust/Slint application combines live departures, NFC card aliases, the configured success sound, and USB photos. No Python/Node installation or spare Pi server is required.
- Reuses the existing private `config/transit.json` contract. Train/bus mode, polling, timezone, paging, cache limits and backoff live in private display settings, based on `pm3/display.example.json`.
- Three large departure rows with route colors, destination, configured direction, and predicted/scheduled indication. Remaining results page automatically, by default every ten seconds, so no touchscreen is required.
- Background requests use the installed curl with certificate verification, no redirects, a 12-second timeout and a 2 MiB response cap. API_KEY travels through stdin rather than process arguments. Responses are parsed into typed Rust structures; error logs omit URLs, keys, stop IDs and destinations.
- Only the selected mode is fetched, with one request per configured stop each cycle. Default interval is 60 seconds after completion. Refresh failures back off to 300 seconds and keep the previous complete snapshot. Failed-stop partial refreshes do not replace it. Cached results are marked, then expire at the configured maximum age. These failure paths have not been fault-injected in this turn.
- Countdowns and the Pacific clock use the API's `currentTime` plus monotonic elapsed time. The incorrect OS clock, timezone and RTC were not changed. TLS still relies on the OS date being within the server certificate's validity period; correct system time synchronization remains subsequent work.
- The reader and renderer remain on the existing bounded loop; network work runs in a separate worker. The existing card action covers the live board with photos and returns to the current arrival cache after its session. Requests continue during the slideshow.
- Audio pop remains parked. No further audio changes were made during this integration.

## Deployment and observations

`pm3/deploy-live.sh root@<device-host> 3600` compiles without running tests, creates ignored local runtime settings, copies only API_KEY from `.env`, and validates staged settings on-device before stopping the current service. The existing card registry and USB/photo action remain in place. Private runtime files are mode 0600; card and location configuration stay out of Git. Previous binary/config files are retained as limited rollback copies. The deployment is not fully transactional if a later transfer/startup step fails.

The ARM build completed successfully. No new tests were added or run for this integration turn. Normal deployment checks showed:

- Both pre-stop and pre-launch configuration validation succeeded.
- The transient service was active, and reader polling configuration was accepted.
- The initial authenticated fetch returned ten departures.
- The webcam preview showed live arrival rows, predicted indicators and automatic paging (page 3 of 4).
- Runtime file permissions were confirmed as 0600 without printing their contents.
- The user confirmed a physical selected-card tap starts the slideshow and returns to live arrivals. Runtime logs showed a successful arrival refresh during the photo sequence, session completion, and another successful refresh afterward.

## Remaining checkpoints

1. Capture full recovery backups and define rollback/recovery before permanent startup changes. The current launch remains a one-hour transient service; it does not install automatic startup.
2. Establish system-wide UTC/time synchronization and validate cold boot, network loss, API errors, stale cache expiry, missing USB, and audio failure when requested.
3. Decide whether remote settings/photo management is useful. A spare Pi can later serve media/configuration, but the initial integration operates directly on the PM3.

The latest request is arrivals while idle. The previous Pi screen-blanking schedule is not applied; PM3 display power controls remain separate work.
