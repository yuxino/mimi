interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  "aria-label"?: string;
}

/** A macOS-style toggle switch used by the tray panel and settings window. */
export function Switch({
  checked,
  onChange,
  disabled = false,
  ...rest
}: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      aria-label={rest["aria-label"]}
      style={{
        position: "relative",
        width: 40,
        height: 24,
        borderRadius: 12,
        border: "none",
        background: checked ? "#3478F0" : "rgba(255,255,255,0.22)",
        cursor: disabled ? "default" : "pointer",
        opacity: disabled ? 0.5 : 1,
        transition: "background 160ms ease",
        flexShrink: 0,
      }}
    >
      <span
        style={{
          position: "absolute",
          top: 2,
          left: checked ? 18 : 2,
          width: 20,
          height: 20,
          borderRadius: "50%",
          background: "#ffffff",
          transition: "left 160ms ease",
        }}
      />
    </button>
  );
}
