import { Clock, Filter, Code2, RefreshCw } from "lucide-react";
import { usePhase8 } from "@/contexts/Phase8Context";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export type TimePreset = "5m" | "15m" | "1h" | "6h" | "24h";

const PRESETS: { value: TimePreset; label: string }[] = [
  { value: "5m", label: "Last 5 minutes" },
  { value: "15m", label: "Last 15 minutes" },
  { value: "1h", label: "Last 1 hour" },
  { value: "6h", label: "Last 6 hours" },
  { value: "24h", label: "Last 24 hours" },
];

export function nsFromPreset(preset: TimePreset): { start: number; end: number } {
  const end = Math.floor(Date.now() * 1e6);
  const mul: Record<TimePreset, number> = {
    "5m": 5 * 60,
    "15m": 15 * 60,
    "1h": 3600,
    "6h": 6 * 3600,
    "24h": 24 * 3600,
  };
  const sec = mul[preset];
  const start = end - sec * 1e9;
  return { start, end };
}

interface TopBarProps {
  timePreset?: TimePreset;
  onTimePresetChange?: (preset: TimePreset) => void;
  onRefresh?: () => void;
  refreshing?: boolean;
}

export function TopBar(props: TopBarProps) {
  const phase8 = usePhase8();
  const timePreset = props.timePreset ?? phase8?.timePreset ?? "1h";
  const onTimePresetChange = props.onTimePresetChange ?? phase8?.setTimePreset;
  const onRefresh = props.onRefresh ?? phase8?.triggerRefresh;
  const refreshing = props.refreshing ?? phase8?.refreshing ?? false;
  return (
    <header className="flex h-12 items-center justify-between border-b border-border bg-card px-4 gap-3">
      <div className="flex items-center gap-2 flex-1">
        <Button variant="ghost" size="sm" className="h-8 gap-1.5 text-muted-foreground hover:text-foreground">
          <Filter className="h-3.5 w-3.5" />
          <span className="text-xs">Filters</span>
        </Button>
        <Button variant="ghost" size="sm" className="h-8 gap-1.5 text-muted-foreground hover:text-foreground">
          <Code2 className="h-3.5 w-3.5" />
        </Button>
        <div className="flex-1 mx-4">
          <input
            type="text"
            placeholder="Add filters..."
            className="h-8 w-full max-w-lg rounded-md border border-border bg-background px-3 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
      </div>

      <div className="flex items-center gap-2">
        <Select
          value={timePreset}
          onValueChange={(v) => onTimePresetChange?.(v as TimePreset)}
        >
          <SelectTrigger className="h-8 w-[150px] border-border bg-background text-xs">
            <Clock className="h-3.5 w-3.5 mr-1.5 text-muted-foreground" />
            <SelectValue />
          </SelectTrigger>
          <SelectContent className="bg-popover border-border">
            {PRESETS.map((p) => (
              <SelectItem key={p.value} value={p.value}>
                {p.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 text-muted-foreground hover:text-foreground"
          onClick={onRefresh}
          disabled={refreshing}
        >
          <RefreshCw className={refreshing ? "animate-spin h-3.5 w-3.5" : "h-3.5 w-3.5"} />
        </Button>
      </div>
    </header>
  );
}
