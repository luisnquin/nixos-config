import { readFile } from "node:fs/promises";
import { homedir } from "node:os";
import { basename, join } from "node:path";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";

type UsageWindow = { used_percent?: number };
type UsageResponse = {
  rate_limit?: { primary_window?: UsageWindow; secondary_window?: UsageWindow };
};

function formatWindow(window: UsageWindow | undefined, label: string): string {
  if (typeof window?.used_percent !== "number" || !Number.isFinite(window.used_percent)) return "";
  return `${label} ${Math.round(window.used_percent)}% used`;
}

async function getRateLimits(): Promise<string> {
  const agentDir = process.env.PI_CODING_AGENT_DIR ?? join(homedir(), ".pi", "agent");
  const auth = JSON.parse(await readFile(join(agentDir, "auth.json"), "utf8")) as {
    openai?: { type?: string; access?: string };
  };
  if (auth.openai?.type !== "oauth" || !auth.openai.access) return "";

  const response = await fetch("https://chatgpt.com/backend-api/wham/usage", {
    headers: { Authorization: `Bearer ${auth.openai.access}` },
    signal: AbortSignal.timeout(5000),
  });
  if (!response.ok) throw new Error(`Usage request failed: ${response.status}`);
  const usage = (await response.json()) as UsageResponse;
  const primary = usage.rate_limit?.primary_window;
  const secondary = usage.rate_limit?.secondary_window;
  const windows = [
    formatWindow(primary, "5h"),
    formatWindow(secondary, "weekly"),
  ].filter(Boolean).join("  ·  ");
  if (!windows) throw new Error("Usage response has no rate-limit windows");
  return windows;
}

export default function (pi: ExtensionAPI) {
  let stopPolling: (() => void) | undefined;
  let refreshLimits: (() => void) | undefined;

  pi.on("session_start", (_event, ctx) => {
    if (ctx.mode !== "tui") return;

    let limits = "";
    let active = true;
    let refresh: (() => void) | undefined;
    const poll = async () => {
      if (ctx.model?.provider !== "openai") {
        limits = "";
        refresh?.();
        return;
      }
      let result: string;
      try {
        result = await getRateLimits();
      } catch {
        result = "quota unavailable";
      }
      if (active && ctx.model?.provider === "openai") {
        limits = result;
        refresh?.();
      }
    };
    const timer = setInterval(() => void poll(), 60_000);
    timer.unref();
    stopPolling = () => {
      active = false;
      clearInterval(timer);
    };
    refreshLimits = () => void poll();
    void poll();

    ctx.ui.setFooter((tui, theme, data) => {
      refresh = () => tui.requestRender();
      const unsubscribe = data.onBranchChange(() => tui.requestRender());

      return {
        dispose: unsubscribe,
        invalidate() {},
        render(width: number): string[] {
          const branch = data.getGitBranch();
          const usage = ctx.getContextUsage();
          const context = usage?.percent == null ? "" : `ctx ${Math.round(usage.percent)}%`;
          const left = theme.fg("dim", [basename(ctx.cwd), branch, context].filter(Boolean).join("  ·  "));
          const right = theme.fg("dim", [ctx.model?.id ?? "no model", pi.getThinkingLevel(), ctx.model?.provider === "openai" ? limits : ""].filter(Boolean).join("  ·  "));
          const available = width - visibleWidth(right) - 2;
          if (available <= 0) return [truncateToWidth(right, width)];
          const clipped = truncateToWidth(left, available);
          const gap = " ".repeat(Math.max(2, width - visibleWidth(clipped) - visibleWidth(right)));
          return [clipped + gap + right];
        },
      };
    });
  });

  pi.on("model_select", () => refreshLimits?.());
  pi.on("session_shutdown", () => {
    stopPolling?.();
    stopPolling = undefined;
    refreshLimits = undefined;
  });
}
