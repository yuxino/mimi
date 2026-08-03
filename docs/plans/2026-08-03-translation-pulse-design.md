# Translation pulse

Mimi reports translation activity from the real high-quality request lifecycle.
The high-quality client emits a translation-started event immediately before each
Qwen-MT Plus request. Session state remains pending until the corresponding final
translation or an error arrives, and resets during stop or reconnect.

The overlay uses that explicit state to change its status to “正在翻译”. The existing
three-bar indicator gains a larger purple pulse: a soft inner glow and an expanding
ring. The same indicator appears in both the expanded language badge and the compact
floating bar. Reduced-motion mode shows a static purple glow instead of animation.

Tests verify the pending-state lifecycle. A seeded UI mode displays the translating
state for visual verification without sending audio to an external service.
