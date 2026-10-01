# Mimi compact pulse style regression

Before: PR #88 `b9d124c220cb90e43ef98618d13e247d50de03b4`.
After candidate: `8e4432efd2a1dd924fccedd2bbd2ed84cf876ebc`.

Actual Mac headless Chromium screenshot, 1440 × 1040. The compact capsule
before component is loaded from exact b9d124c source (only import paths rebased).
The candidate forwards the stored pulse style. Support modules, font, CSS and
synthetic state share the same baseline. This is a real React/CSS component
comparison, not a native application window, provider session, or audio test.

The top three rows show the same selection in the standalone indicator, old
compact capsule, and candidate compact capsule. The lower matrix shows all
three styles in seven explicitly synthetic phases with motion switched off.

41 actual browser assertions pass. They include persistent CSS animation
instances across working phases; paused/error/idle settling then frozen clocks;
same-clock resume; explicit off; default system reduced motion; independent
pulse and subtitle motion preferences; and repeated settings category changes.
See results.json. The screenshot does not itself prove animation timing.

Frontend: 250 tests pass, typecheck/production build pass, lint zero errors with
one existing SoftwareUpdate refresh warning. New exact-head remote CI and a
native package containing this minimal follow-up are distinct checks.

Reproduce: select A or B in Settings → Subtitles → Pulse style, then open the
compact subtitle control entry. Both the settings preview and compact 18 px
indicator should use the chosen sound shape; switching back restores the ring.
Repeat with expanded controls and motion off. Native follow-up remains pending.

No real credentials, subtitles, recordings, or personal browser profile used.

## Formal authentication is separate

The prior exact b9d124c formal Mac candidate was strictly verified and matched
the old Bundle ID, certificate and full DR. Its first normal startup requested
login-Keychain authentication. It was normally exited and the complete original
formal app restored and verified; non-secret configuration did not change.
No authentication, ACL/TCC change or paid capture was automated. Credential
read/two cold restarts remain unverified and issue #83 stays open. This does not
claim credential loss or that every future upgrade must prompt.
