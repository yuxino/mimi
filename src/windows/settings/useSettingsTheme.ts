import { useEffect, useState } from "react";

export type SettingsTheme = "system" | "light" | "dark";
const STORAGE_KEY = "mimi.settings-theme";

function readTheme(): SettingsTheme {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "light" || saved === "dark") return saved;
  } catch {
    // A blocked preference store must not prevent opening settings.
  }
  return "system";
}

export function useSettingsTheme() {
  const [theme, setTheme] = useState<SettingsTheme>(readTheme);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );
  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(query.matches);
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);

  const resolvedTheme =
    theme === "system" ? (systemDark ? "dark" : "light") : theme;
  useEffect(() => {
    document.body.style.backgroundColor =
      resolvedTheme === "light" ? "#fafafa" : "#0b0b0b";
    return () => {
      document.body.style.backgroundColor = "";
    };
  }, [resolvedTheme]);

  return {
    theme,
    resolvedTheme,
    changeTheme: (value: SettingsTheme) => {
      setTheme(value);
      try {
        localStorage.setItem(STORAGE_KEY, value);
      } catch {
        // Keep the session choice when preference storage is unavailable.
      }
    },
  };
}
