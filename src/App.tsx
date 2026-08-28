import { getCurrentWindow } from "@tauri-apps/api/window";
import { lazy, Suspense, useEffect, useState } from "react";
import { isTauri } from "./lib/ipc";
import { useStore } from "./lib/store";

const OverlayWindow = lazy(() =>
  import("./windows/overlay/OverlayWindow").then((module) => ({
    default: module.OverlayWindow,
  })),
);
const OverlayControlWindow = lazy(() =>
  import("./windows/overlay-control/OverlayControlWindow").then((module) => ({
    default: module.OverlayControlWindow,
  })),
);
const SettingsView = lazy(() =>
  import("./windows/settings/SettingsView").then((module) => ({
    default: module.SettingsView,
  })),
);
const TrayPanel = lazy(() =>
  import("./windows/tray-panel/TrayPanel").then((module) => ({
    default: module.TrayPanel,
  })),
);

type WindowLabel = "overlay" | "overlay-control" | "tray-panel" | "settings";

/**
 * Every window loads the shared entry, then lazily loads only the component
 * matching its label: "overlay" (floating subtitles), "tray-panel" (menu-bar
 * style control panel), "settings" (main settings window), or
 * "overlay-control" (the child status island and control panel). Outside
 * Tauri a `?window=` query parameter selects the preview (defaults to
 * "settings").
 */
export default function App() {
  const [label] = useState<WindowLabel>(resolveInitialLabel);
  const init = useStore((state) => state.init);

  useEffect(() => {
    void init();
  }, [init]);

  useEffect(() => {
    document.body.classList.toggle("settings-body", label === "settings");
  }, [label]);

  let windowContent;
  if (label === "overlay") windowContent = <OverlayWindow />;
  else if (label === "overlay-control") windowContent = <OverlayControlWindow />;
  else if (label === "tray-panel") windowContent = <TrayPanel />;
  else windowContent = <SettingsView />;

  return <Suspense fallback={null}>{windowContent}</Suspense>;
}

function resolveInitialLabel(): WindowLabel {
  if (isTauri) {
    return getCurrentWindow().label as WindowLabel;
  }
  const param = new URLSearchParams(window.location.search).get("window");
  return param === "overlay" ||
    param === "overlay-control" ||
    param === "tray-panel" ||
    param === "settings"
    ? param
    : "settings";
}
