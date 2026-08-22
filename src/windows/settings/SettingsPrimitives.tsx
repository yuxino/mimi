import type { ReactNode } from "react";
import { Icon, type IconName } from "../../components/Icon";

export function SettingsSection({
  id,
  icon,
  title,
  description,
  action,
  children,
}: {
  id: string;
  icon: IconName;
  title: string;
  description: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section id={id} className="settings-card" aria-labelledby={`${id}-title`}>
      <header className="settings-card__header">
        <span className="settings-card__icon" aria-hidden="true">
          <Icon name={icon} />
        </span>
        <span className="settings-card__heading">
          <h2 id={`${id}-title`}>{title}</h2>
          <p>{description}</p>
        </span>
        {action && <span className="settings-card__action">{action}</span>}
      </header>
      <div className="settings-card__body">{children}</div>
    </section>
  );
}

export function SettingsRow({
  label,
  description,
  children,
  align = "center",
}: {
  label: string;
  description?: string;
  children: ReactNode;
  align?: "center" | "start";
}) {
  return (
    <div className={`settings-row settings-row--${align}`}>
      <span className="settings-row__copy">
        <span className="settings-row__label">{label}</span>
        {description && (
          <span className="settings-row__description">{description}</span>
        )}
      </span>
      <span className="settings-row__control">{children}</span>
    </div>
  );
}

export function SettingsSelect({
  value,
  disabled = false,
  onChange,
  options,
  label,
}: {
  value: string;
  disabled?: boolean;
  onChange: (value: string) => void;
  options: readonly { value: string; label: string }[];
  label: string;
}) {
  return (
    <span className="settings-select-wrap">
      <select
        className="settings-select"
        value={value}
        disabled={disabled}
        aria-label={label}
        onChange={(event) => onChange(event.target.value)}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      <Icon name="chevron-down" />
    </span>
  );
}

export function InlineFeedback({
  tone,
  icon,
  children,
}: {
  tone: "success" | "error" | "info";
  icon?: IconName;
  children: ReactNode;
}) {
  return (
    <p
      className="settings-feedback"
      data-tone={tone}
      role={tone === "error" ? "alert" : "status"}
    >
      <Icon
        name={
          icon ??
          (tone === "success"
            ? "checkmark-circle"
            : tone === "error"
              ? "exclamation-triangle"
              : "sparkles")
        }
      />
      <span>{children}</span>
    </p>
  );
}
