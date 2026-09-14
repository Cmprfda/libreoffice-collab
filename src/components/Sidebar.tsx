/** Left navigation, modelled on the Windows 11 Settings app. */
import { useI18n } from "../i18n";
import { HomeIcon, SettingsIcon } from "./Icons";

export type Route = "dashboard" | "settings";

export function Sidebar({
  route,
  onNavigate,
  version,
}: {
  route: Route;
  onNavigate: (route: Route) => void;
  version: string;
}) {
  const { t } = useI18n();

  const items: { id: Route; label: string; icon: JSX.Element }[] = [
    { id: "dashboard", label: t("nav.dashboard"), icon: <HomeIcon size={18} /> },
    {
      id: "settings",
      label: t("nav.settings"),
      icon: <SettingsIcon size={18} />,
    },
  ];

  return (
    <nav className="sidebar" aria-label={t("app.name")}>
      {items.map((item) => (
        <button
          key={item.id}
          className={`nav-item${route === item.id ? " nav-item--active" : ""}`}
          aria-current={route === item.id ? "page" : undefined}
          onClick={() => onNavigate(item.id)}
        >
          {item.icon}
          <span>{item.label}</span>
        </button>
      ))}

      <div className="sidebar-footer">
        {t("settings.installedVersion", { version })}
      </div>
    </nav>
  );
}
