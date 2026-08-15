// Lightweight browser smoke test for the Vite previews.
// Usage: npm run e2e
//
// This intentionally tests the plain-vite preview (not the Tauri shell) so it
// can run in CI without a desktop display. It launches the Vite dev server,
// opens every window preview, and checks for runtime errors, overflow, and the
// core overlay interaction.

import { spawn } from "node:child_process";
import { chromium } from "playwright-core";

const PORT = 1420;
const BASE = `http://localhost:${PORT}`;

function waitForServer(url, timeoutMs = 20_000) {
  const started = Date.now();
  return new Promise((resolve, reject) => {
    const tick = async () => {
      try {
        const response = await fetch(url);
        if (response.ok) return resolve();
      } catch {
        // Server not up yet.
      }
      if (Date.now() - started > timeoutMs) {
        return reject(new Error(`Timed out waiting for ${url}`));
      }
      setTimeout(tick, 200);
    };
    tick();
  });
}

const vite = spawn("npm", ["run", "dev"], {
  stdio: "ignore",
  detached: false,
});

function stopVite() {
  if (!vite.killed) vite.kill("SIGTERM");
}

try {
  await waitForServer(BASE);

  const browser = await chromium.launch({ headless: true });
  const windows = ["settings", "overlay", "tray-panel", "language-popover"];

  for (const windowName of windows) {
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });

    await page.goto(`${BASE}/?window=${windowName}`, {
      waitUntil: "networkidle",
    });
    await page.waitForTimeout(120);

    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth,
    );
    const bodyText = (await page.locator("body").innerText()).trim();

    if (errors.length > 0) {
      throw new Error(`${windowName} page errors: ${errors.join("; ")}`);
    }
    if (overflow) {
      throw new Error(`${windowName} page has horizontal overflow`);
    }
    if (bodyText.length === 0) {
      throw new Error(`${windowName} page rendered empty`);
    }

    await page.close();
  }

  // Overlay double-click collapse/expand behavior.
  {
    const page = await browser.newPage();
    await page.goto(`${BASE}/?window=overlay`, { waitUntil: "networkidle" });
    await page.waitForTimeout(120);

    const handle = page.locator('[data-testid="drag-handle"]');
    if ((await handle.count()) !== 1) {
      throw new Error("Overlay drag handle not found");
    }
    await handle.dblclick();
    await page.waitForTimeout(200);
    const expandButton = page.locator('[data-testid="expand-subtitles"]');
    if ((await expandButton.count()) !== 1) {
      throw new Error("Double-click did not collapse the overlay");
    }
    await expandButton.click();
    await page.waitForTimeout(200);
    if ((await handle.count()) !== 1) {
      throw new Error("Expand did not restore the overlay drag handle");
    }
    await page.close();
  }

  // Settings API key Enter-to-save behavior.
  {
    const page = await browser.newPage();
    await page.goto(`${BASE}/?window=settings`, { waitUntil: "networkidle" });
    const input = page.locator('input[type="password"]');
    await input.fill("sk-e2e-test");
    await input.press("Enter");
    await page.waitForTimeout(150);
    const text = await page.locator("body").innerText();
    if (!text.includes("Credentials saved securely.") && !text.includes("凭证已安全保存。")) {
      throw new Error("Enter did not save API key credentials");
    }
    await page.close();
  }

  await browser.close();
  console.log("E2E smoke passed");
} finally {
  stopVite();
}
