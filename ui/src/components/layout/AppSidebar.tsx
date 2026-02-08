import { NavLink, useLocation } from "react-router-dom";
import {
  Cpu,
  Flame,
  BarChart3,
  GitCompare,
  LayoutDashboard,
  Bell,
  Settings,
  Scan,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

const navItems = [
  { icon: LayoutDashboard, label: "Dashboard", to: "/" },
  { icon: Flame, label: "Flamegraph", to: "/flamegraph" },
  { icon: BarChart3, label: "Top Functions", to: "/functions" },
  { icon: GitCompare, label: "Comparison", to: "/comparison" },
  { icon: Cpu, label: "Timeline", to: "/timeline" },
];

const bottomItems = [
  { icon: Bell, label: "Alerts", to: "/alerts" },
  { icon: Settings, label: "Settings", to: "/settings" },
];

export function AppSidebar() {
  const location = useLocation();

  const renderItem = (item: typeof navItems[0]) => {
    const isActive = location.pathname === item.to;
    return (
      <Tooltip key={item.to} delayDuration={0}>
        <TooltipTrigger asChild>
          <NavLink
            to={item.to}
            className={cn(
              "flex h-10 w-10 items-center justify-center rounded-md transition-colors",
              isActive
                ? "bg-primary/15 text-primary"
                : "text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
            )}
          >
            <item.icon className="h-5 w-5" />
          </NavLink>
        </TooltipTrigger>
        <TooltipContent side="right" className="bg-popover text-popover-foreground border-border">
          {item.label}
        </TooltipContent>
      </Tooltip>
    );
  };

  return (
    <aside className="flex h-full w-14 flex-col items-center border-r border-sidebar-border bg-sidebar py-3 gap-1">
      <div className="flex h-10 w-10 flex-col items-center justify-center mb-4" title="Aperture Profiler">
        <Scan className="h-7 w-7 text-primary" />
        <span className="text-[8px] text-primary/70 mt-0.5 font-semibold tracking-wider">APT</span>
      </div>

      <nav className="flex flex-1 flex-col items-center gap-1">
        {navItems.map(renderItem)}
      </nav>

      <div className="flex flex-col items-center gap-1">
        {bottomItems.map(renderItem)}
      </div>
    </aside>
  );
}
