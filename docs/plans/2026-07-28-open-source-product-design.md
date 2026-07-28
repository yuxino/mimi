# Open-source Product Design

## Goal

Turn mimi into a recognizable, trustworthy open-source macOS product that a new
user can build, configure with their own Alibaba Cloud credentials, and use
without reading implementation notes.

## Brand

The product uses a cat-eared subtitle-bubble mascot: a charcoal speech bubble,
two cat ears, warm white facial details, and a mint live-audio accent. The mark
must remain legible as a small macOS icon and friendly without becoming
childish. A transparent master PNG supplies README branding, while an ICNS
derivative ships in the app bundle.

## Documentation

`README.md` is the default Simplified Chinese entry point and links to a complete
`README_EN.md`. Both documents cover the same product promise, requirements,
credential setup, build commands, first-run permissions, daily controls,
architecture, privacy, troubleshooting, contribution path, and license.
Credential links point to current official Alibaba Cloud documentation for the
China (Beijing) region.

Screenshots use the real packaged interface in a deterministic UI-test state.
All credential fields contain explicit non-secret placeholders. The primary
screenshot demonstrates readable translated history and overlay controls; a
second screenshot documents configuration.

## Open-source Readiness

The repository includes an MIT license, bilingual contribution guidance, a
security policy, a practical `.gitignore`, deterministic tests, and a packaging
script that embeds and signs the cat icon. Before publication, the complete
tracked tree and screenshots are scanned for credentials and personal data.
The clean-clone commands in both READMEs must build and package successfully.
