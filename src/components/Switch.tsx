interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  "aria-label"?: string;
}

/** Platform-neutral toggle used by settings surfaces. */
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
      className={`mimi-switch${checked ? " is-checked" : ""}`}
    >
      <span className="mimi-switch__thumb" aria-hidden="true" />
    </button>
  );
}
