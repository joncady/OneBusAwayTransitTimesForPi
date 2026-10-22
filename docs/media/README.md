# README demo media

The project owner supplied the nine-second PM3 recording and selected it for the
README. It shows U District live arrivals and several card-triggered album
selections. The photo-session timeout occurs after the recorded clip ends.

- `pm3-demo.mp4`: full-frame, silent H.264/SDR copy, 540 × 960 at 15 fps.
- `pm3-demo.gif`: looping 320-pixel-wide preview at 6 fps for inline README display.

Printed identifiers on the two blue ORCA cards are selectively pixelated using
tracked number-field regions. The terminal, card designs and other cards remain
visible. Recording/location metadata and the audio track are omitted. The
original HEVC/HDR recording and temporary processing files stay outside Git.

The README embeds the GIF and links to the MP4 with relative paths, so it works
in a cloned checkout without a separate asset-host upload. GitHub recommends
[H.264 for video compatibility](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files).

These approved demo excerpts are separate from the private album originals;
individual personal photos, card registries and private configurations are not
repository assets.
