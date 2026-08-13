import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { isTauri } from "./lib/ipc";
import { useStore } from "./lib/store";
import { OverlayWindow } from "./windows/overlay/OverlayWindow";
import { SettingsView } from "./windows/settings/SettingsView";
import { TrayPanel } from "./windows/tray-panel/TrayPanel";

type WindowLabel = "overlay" | "tray-panel" | "settings";

/**
 * Every window loads the same bundle and renders the component matching its
 * label: "overlay" (floating subtitles), "tray-panel" (menu-bar style control
 * panel), or "settings" (main settings window). Outside Tauri a `?window=`
 * query parameter selects the preview (defaults to "settings").
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

  if (label === "overlay") return <OverlayWindow />;
  if (label === "tray-panel") return <TrayPanel />;
  return <SettingsView />;
}

function resolveInitialLabel(): WindowLabel {
  if (isTauri) {
    return getCurrentWindow().label as WindowLabel;
  }
  const param = new URLSearchParams(window.location.search).get("window");
  return param === "overlay" || param === "tray-panel" ? param : "settings";
}
