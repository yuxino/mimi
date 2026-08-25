import { Icon, type IconName } from "../../components/Icon";

interface ControlButtonProps {
  icon: IconName;
  label: string;
  onClick: () => void;
  "data-testid"?: string;
}

/** 24×24 rounded control with a 10pt icon (matches `OverlayControlButton`). */
export function ControlButton({
  icon,
  label,
  onClick,
  "data-testid": dataTestId,
}: ControlButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={label}
      aria-label={label}
      data-testid={dataTestId}
      className="ux-hover flex items-center justify-center"
      style={{
        width: 24,
        height: 24,
        borderRadius: 8,
        background: "rgba(0, 0, 0, 0.38)",
        color: "rgba(255, 255, 255, 0.8)",
        fontSize: 10,
        border: "1px solid rgba(255, 255, 255, 0.08)",
        padding: 0,
        cursor: "pointer",
      }}
    >
      <Icon name={icon} />
    </button>
  );
}
