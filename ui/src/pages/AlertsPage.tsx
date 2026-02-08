import { useState } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import {
  useAlertRulesQuery,
  useAlertHistoryQuery,
  useCreateAlertMutation,
  useDeleteAlertMutation,
  useToggleAlertMutation,
  useEvaluateAlertsMutation,
} from "@/api/queries";
import type {
  AlertMetric,
  AlertOperator,
  AlertSeverity,
  AlertRule,
  AlertEvent,
} from "@/api/types";
import {
  Bell,
  Plus,
  Trash2,
  Play,
  ToggleLeft,
  ToggleRight,
  AlertTriangle,
  Info,
  XCircle,
} from "lucide-react";

const METRIC_OPTIONS: { value: AlertMetric; label: string }[] = [
  { value: "buffer_utilization", label: "Buffer Utilization" },
  { value: "push_error_rate", label: "Push Error Rate" },
  { value: "push_errors_total", label: "Push Errors (total)" },
  { value: "clickhouse_flush_errors", label: "ClickHouse Flush Errors" },
  { value: "clickhouse_pending_rows", label: "ClickHouse Pending Rows" },
  { value: "event_throughput", label: "Event Throughput" },
];

const OPERATOR_OPTIONS: { value: AlertOperator; label: string }[] = [
  { value: "gt", label: ">" },
  { value: "gte", label: ">=" },
  { value: "lt", label: "<" },
  { value: "lte", label: "<=" },
  { value: "eq", label: "==" },
];

const SEVERITY_OPTIONS: { value: AlertSeverity; label: string }[] = [
  { value: "info", label: "Info" },
  { value: "warning", label: "Warning" },
  { value: "critical", label: "Critical" },
];

const metricLabel = (m: AlertMetric) =>
  METRIC_OPTIONS.find((o) => o.value === m)?.label ?? m;

const operatorLabel = (o: AlertOperator) =>
  OPERATOR_OPTIONS.find((op) => op.value === o)?.label ?? o;

const severityStyle: Record<AlertSeverity, { bg: string; text: string; border: string }> = {
  info: { bg: "bg-primary/10", text: "text-primary", border: "border-primary/30" },
  warning: { bg: "bg-amber-500/10", text: "text-amber-600 dark:text-amber-400", border: "border-amber-500/30" },
  critical: { bg: "bg-destructive/10", text: "text-destructive", border: "border-destructive/30" },
};

function SeverityIcon({ severity, className }: { severity: AlertSeverity; className?: string }) {
  switch (severity) {
    case "critical":
      return <XCircle className={className} />;
    case "warning":
      return <AlertTriangle className={className} />;
    default:
      return <Info className={className} />;
  }
}

function CreateRuleForm({ onCreated }: { onCreated: () => void }) {
  const createMutation = useCreateAlertMutation();
  const [name, setName] = useState("");
  const [metric, setMetric] = useState<AlertMetric>("buffer_utilization");
  const [operator, setOperator] = useState<AlertOperator>("gt");
  const [threshold, setThreshold] = useState("0.9");
  const [severity, setSeverity] = useState<AlertSeverity>("warning");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    createMutation.mutate(
      {
        name: name.trim(),
        metric,
        operator,
        threshold: parseFloat(threshold),
        severity,
      },
      {
        onSuccess: () => {
          setName("");
          setThreshold("0.9");
          onCreated();
        },
      },
    );
  };

  return (
    <form onSubmit={handleSubmit} className="rounded-md border border-border bg-card p-4 space-y-3">
      <h3 className="text-sm font-medium text-foreground flex items-center gap-2">
        <Plus className="h-4 w-4" />
        New Alert Rule
      </h3>
      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="text-[11px] text-muted-foreground block mb-1">Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. High buffer usage"
            className="w-full rounded-md border border-border bg-background px-2.5 py-1.5 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
        <div>
          <label className="text-[11px] text-muted-foreground block mb-1">Severity</label>
          <select
            value={severity}
            onChange={(e) => setSeverity(e.target.value as AlertSeverity)}
            className="w-full rounded-md border border-border bg-background px-2.5 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          >
            {SEVERITY_OPTIONS.map((s) => (
              <option key={s.value} value={s.value}>{s.label}</option>
            ))}
          </select>
        </div>
      </div>
      <div className="grid grid-cols-3 gap-3">
        <div>
          <label className="text-[11px] text-muted-foreground block mb-1">Metric</label>
          <select
            value={metric}
            onChange={(e) => setMetric(e.target.value as AlertMetric)}
            className="w-full rounded-md border border-border bg-background px-2.5 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          >
            {METRIC_OPTIONS.map((m) => (
              <option key={m.value} value={m.value}>{m.label}</option>
            ))}
          </select>
        </div>
        <div>
          <label className="text-[11px] text-muted-foreground block mb-1">Operator</label>
          <select
            value={operator}
            onChange={(e) => setOperator(e.target.value as AlertOperator)}
            className="w-full rounded-md border border-border bg-background px-2.5 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
          >
            {OPERATOR_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        </div>
        <div>
          <label className="text-[11px] text-muted-foreground block mb-1">Threshold</label>
          <input
            type="number"
            step="any"
            value={threshold}
            onChange={(e) => setThreshold(e.target.value)}
            className="w-full rounded-md border border-border bg-background px-2.5 py-1.5 text-xs text-foreground font-mono focus:outline-none focus:ring-1 focus:ring-primary"
          />
        </div>
      </div>
      <button
        type="submit"
        disabled={!name.trim() || createMutation.isPending}
        className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50 transition-colors"
      >
        {createMutation.isPending ? "Creating…" : "Create Rule"}
      </button>
      {createMutation.isError && (
        <p className="text-xs text-destructive">{createMutation.error?.message}</p>
      )}
    </form>
  );
}

function RuleCard({
  rule,
  onToggle,
  onDelete,
}: {
  rule: AlertRule;
  onToggle: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const style = severityStyle[rule.severity];
  return (
    <div
      className={`rounded-md border px-3 py-2.5 flex items-center gap-3 transition-opacity ${style.border} ${
        rule.enabled ? "" : "opacity-50"
      }`}
    >
      <SeverityIcon severity={rule.severity} className={`h-4 w-4 shrink-0 ${style.text}`} />
      <div className="flex-1 min-w-0">
        <div className="text-xs font-medium text-foreground truncate">{rule.name}</div>
        <div className="text-[11px] text-muted-foreground mt-0.5">
          {metricLabel(rule.metric)} {operatorLabel(rule.operator)}{" "}
          <span className="font-mono">{rule.threshold}</span>
        </div>
      </div>
      <span className={`text-[10px] px-1.5 py-0.5 rounded ${style.bg} ${style.text}`}>
        {rule.severity}
      </span>
      <button
        onClick={() => onToggle(rule.id)}
        className="text-muted-foreground hover:text-foreground transition-colors"
        title={rule.enabled ? "Disable" : "Enable"}
      >
        {rule.enabled ? (
          <ToggleRight className="h-5 w-5 text-primary" />
        ) : (
          <ToggleLeft className="h-5 w-5" />
        )}
      </button>
      <button
        onClick={() => onDelete(rule.id)}
        className="text-muted-foreground hover:text-destructive transition-colors"
        title="Delete rule"
      >
        <Trash2 className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

function HistoryEntry({ event }: { event: AlertEvent }) {
  const style = severityStyle[event.severity];
  const time = new Date(event.fired_at * 1000).toLocaleString();
  return (
    <div className={`rounded-md border px-3 py-2 text-xs ${style.border} ${style.bg}`}>
      <div className="flex items-center gap-2">
        <SeverityIcon severity={event.severity} className={`h-3.5 w-3.5 shrink-0 ${style.text}`} />
        <span className="font-medium text-foreground flex-1 truncate">{event.message}</span>
        <span className="text-[10px] text-muted-foreground shrink-0">{time}</span>
      </div>
    </div>
  );
}

export default function AlertsPage() {
  const [showForm, setShowForm] = useState(false);
  const rulesQuery = useAlertRulesQuery();
  const historyQuery = useAlertHistoryQuery();
  const toggleMutation = useToggleAlertMutation();
  const deleteMutation = useDeleteAlertMutation();
  const evaluateMutation = useEvaluateAlertsMutation();

  const rules = rulesQuery.data ?? [];
  const history = historyQuery.data ?? [];
  const activeCount = rules.filter((r) => r.enabled).length;

  return (
    <AppLayout>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold text-foreground flex items-center gap-2">
            <Bell className="h-5 w-5" />
            Alerts
          </h1>
          <div className="flex items-center gap-2">
            <button
              onClick={() => evaluateMutation.mutate()}
              disabled={evaluateMutation.isPending || rules.length === 0}
              className="inline-flex items-center gap-1.5 rounded-md border border-border bg-card px-3 py-1.5 text-xs text-foreground hover:bg-muted/50 disabled:opacity-50 transition-colors"
              title="Evaluate all rules against current metrics"
            >
              <Play className="h-3.5 w-3.5" />
              {evaluateMutation.isPending ? "Evaluating…" : "Evaluate Now"}
            </button>
            <button
              onClick={() => setShowForm(!showForm)}
              className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
            >
              <Plus className="h-3.5 w-3.5" />
              New Rule
            </button>
          </div>
        </div>

        {(rulesQuery.isError || historyQuery.isError) && (
          <div className="rounded-md border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
            {rulesQuery.error?.message ?? historyQuery.error?.message}
          </div>
        )}

        {evaluateMutation.data && evaluateMutation.data.fired.length > 0 && (
          <div className="rounded-md border border-amber-500/50 bg-amber-500/10 px-4 py-2 text-sm text-amber-700 dark:text-amber-400">
            {evaluateMutation.data.fired.length} alert(s) fired during evaluation.
          </div>
        )}

        {showForm && <CreateRuleForm onCreated={() => setShowForm(false)} />}

        {/* Rules */}
        <div className="rounded-md border border-border bg-card p-4">
          <h2 className="text-sm font-medium text-foreground mb-3">
            Alert Rules
            <span className="ml-2 text-[10px] text-muted-foreground font-normal">
              {activeCount} active / {rules.length} total
            </span>
          </h2>
          {rules.length === 0 ? (
            <p className="text-xs text-muted-foreground py-4 text-center">
              No alert rules configured. Click "New Rule" to create one.
            </p>
          ) : (
            <div className="space-y-2">
              {rules.map((rule) => (
                <RuleCard
                  key={rule.id}
                  rule={rule}
                  onToggle={(id) => toggleMutation.mutate(id)}
                  onDelete={(id) => deleteMutation.mutate(id)}
                />
              ))}
            </div>
          )}
        </div>

        {/* Metric snapshot from last evaluation */}
        {evaluateMutation.data?.snapshot && (
          <div className="rounded-md border border-border bg-card p-4">
            <h2 className="text-sm font-medium text-foreground mb-3">Current Metric Values</h2>
            <div className="grid grid-cols-3 gap-3">
              {Object.entries(evaluateMutation.data.snapshot).map(([key, value]) => (
                <div key={key} className="text-xs">
                  <div className="text-[11px] text-muted-foreground">
                    {metricLabel(key as AlertMetric)}
                  </div>
                  <div className="font-mono text-foreground text-sm">
                    {typeof value === "number" ? value.toFixed(4) : String(value)}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* History */}
        <div className="rounded-md border border-border bg-card p-4">
          <h2 className="text-sm font-medium text-foreground mb-3">
            Alert History
            <span className="ml-2 text-[10px] text-muted-foreground font-normal">
              {history.length} events
            </span>
          </h2>
          {history.length === 0 ? (
            <p className="text-xs text-muted-foreground py-4 text-center">
              No alerts have fired yet. Alerts trigger when rule conditions are met during evaluation.
            </p>
          ) : (
            <div className="space-y-1.5 max-h-80 overflow-auto">
              {history.map((event, i) => (
                <HistoryEntry key={`${event.rule_id}-${event.fired_at}-${i}`} event={event} />
              ))}
            </div>
          )}
        </div>
      </div>
    </AppLayout>
  );
}
