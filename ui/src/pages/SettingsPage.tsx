import { useState } from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { setAuthToken } from "@/api/client";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Shield, Database, Activity, RefreshCw, Check } from "lucide-react";
import { useHealthQuery } from "@/api/queries";

export default function SettingsPage() {
  const healthQuery = useHealthQuery();
  const health = healthQuery.data ?? null;
  const loading = healthQuery.isFetching;
  const error = healthQuery.error?.message ?? null;
  const [token, setToken] = useState("");
  const [tokenSaved, setTokenSaved] = useState(false);

  const handleSaveToken = () => {
    setAuthToken(token || null);
    setTokenSaved(true);
    setTimeout(() => setTokenSaved(false), 2000);
    healthQuery.refetch();
  };

  return (
    <AppLayout>
      <div className="space-y-6 max-w-2xl">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold text-foreground">Settings</h1>
          <Button
            variant="ghost"
            size="sm"
            className="h-8 gap-1.5 text-xs"
            onClick={() => healthQuery.refetch()}
            disabled={loading}
          >
            <RefreshCw className={loading ? "animate-spin h-3.5 w-3.5" : "h-3.5 w-3.5"} />
            Refresh
          </Button>
        </div>

        {/* Connection Status */}
        <div className="rounded-md border border-border bg-card p-4 space-y-3">
          <h2 className="text-sm font-medium text-foreground">Aggregator Connection</h2>
          {error ? (
            <div className="flex items-center gap-2">
              <span className="h-2 w-2 rounded-full bg-destructive" />
              <span className="text-xs text-destructive">{error}</span>
            </div>
          ) : health ? (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <span className={`h-2 w-2 rounded-full ${health.status === "healthy" ? "bg-success" : "bg-warning"}`} />
                <span className="text-xs text-foreground font-medium">
                  {health.status === "healthy" ? "Connected & Healthy" : "Connected (Degraded)"}
                </span>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Shield className="h-3.5 w-3.5" />
                  Push RPCs: {health.push_total_ok} ok / {health.push_total_error} err
                </div>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Activity className="h-3.5 w-3.5" />
                  Events ingested: {health.push_events_total.toLocaleString()}
                </div>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Database className="h-3.5 w-3.5" />
                  Storage: {health.storage_enabled ? "ClickHouse enabled" : "In-memory only"}
                </div>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Activity className="h-3.5 w-3.5" />
                  Buffer: {health.buffer_batches} batches ({(health.buffer_utilization * 100).toFixed(1)}%)
                </div>
              </div>
              {health.storage_enabled && (
                <div className="text-xs text-muted-foreground">
                  ClickHouse: {health.clickhouse_flush_ok} flushes ok, {health.clickhouse_flush_error} errors,{" "}
                  {health.clickhouse_pending_rows} pending rows
                </div>
              )}
            </div>
          ) : (
            <p className="text-xs text-muted-foreground">Connecting…</p>
          )}
        </div>

        {/* Auth Token */}
        <div className="rounded-md border border-border bg-card p-4 space-y-3">
          <h2 className="text-sm font-medium text-foreground">Authentication</h2>
          <p className="text-xs text-muted-foreground">
            If the aggregator requires bearer token authentication (APERTURE_AUTH_TOKEN), enter the token below.
          </p>
          <div className="flex gap-2">
            <Input
              type="password"
              placeholder="Bearer token (leave empty for no auth)"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              className="h-8 text-xs font-mono"
            />
            <Button size="sm" className="h-8 gap-1.5" onClick={handleSaveToken}>
              {tokenSaved ? <Check className="h-3.5 w-3.5" /> : null}
              {tokenSaved ? "Saved" : "Set token"}
            </Button>
          </div>
        </div>

        {/* Environment Info */}
        <div className="rounded-md border border-border bg-card p-4 space-y-3">
          <h2 className="text-sm font-medium text-foreground">Environment</h2>
          <div className="space-y-1 text-xs font-mono text-muted-foreground">
            <div>API endpoint: /api (proxied to aggregator admin port)</div>
            <div>Aggregator gRPC: port 50051 (default)</div>
            <div>Admin HTTP: port 9090 (default)</div>
          </div>
        </div>
      </div>
    </AppLayout>
  );
}
