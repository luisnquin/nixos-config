import { basename } from "node:path";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";

export default function (pi: ExtensionAPI) {
  pi.on("session_start", (_event, ctx) => {
    if (ctx.mode !== "tui") return;

    ctx.ui.setFooter((tui, theme, data) => {
      const unsubscribe = data.onBranchChange(() => tui.requestRender());

      return {
        dispose: unsubscribe,
        invalidate() {},
        render(width: number): string[] {
          const branch = data.getGitBranch();
          const usage = ctx.getContextUsage();
          const context = usage?.percent == null ? "" : `ctx ${Math.round(usage.percent)}%`;
          const left = theme.fg("dim", [basename(ctx.cwd), branch, context].filter(Boolean).join("  ·  "));
          const right = theme.fg("dim", [ctx.model?.id ?? "no model", pi.getThinkingLevel()].join("  ·  "));
          const available = width - visibleWidth(right) - 2;
          if (available <= 0) return [truncateToWidth(right, width)];
          const clipped = truncateToWidth(left, available);
          const gap = " ".repeat(Math.max(2, width - visibleWidth(clipped) - visibleWidth(right)));
          return [clipped + gap + right];
        },
      };
    });
  });
}
