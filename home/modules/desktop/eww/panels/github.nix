{
  config,
  lib,
  pkgs,
  ...
}: let
  githubInfo = pkgs.writeShellApplication {
    name = "eww-github-monitor";
    runtimeInputs = with pkgs; [coreutils jq];
    text = ''
      cache=${lib.escapeShellArg "${config.xdg.cacheHome}/github-monitor/state.json"}
      if [ -r "$cache" ]; then
        cat "$cache"
      else
        jq -cn '{workflows:[],issues:[],errors:[],workflow_count:0,failed_count:0,issue_count:0,updated_at:"waiting for first refresh"}'
      fi
    '';
  };
in {
  yuck = ''
    (defpoll github_data :interval "30s"
      :initial '{"workflows":[],"issues":[],"errors":[],"workflow_count":0,"failed_count":0,"issue_count":0,"updated_at":"loading"}'
      `${lib.getExe githubInfo}`)

    (defwindow github
      :monitor 0
      :geometry (geometry
        :x "235px"
        :y "35px"
        :width "390px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (github-widget))

    (defwidget github-widget []
      (box :class "github-box" :orientation "v" :space-evenly false :spacing 12
        (box :class "github-header" :orientation "h" :space-evenly false :spacing 10
          (label :class "github-mark" :text "")
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "github-title" :halign "start" :text "GitHub watch")
            (label :class "github-meta" :halign "start"
              :text {github_data.failed_count + " failed / " + github_data.workflow_count + " watched · " + github_data.issue_count + " issues"})))
        (box :class "github-section" :orientation "v" :space-evenly false :spacing 5
          (label :class "github-section-title" :halign "start" :text "WATCHED WORKFLOWS")
          (for run in {github_data.workflows}
            (button :class {"github-entry " + run.conclusion} :onclick {"${lib.getExe' pkgs.xdg-utils "xdg-open"} " + run.url}
              (box :orientation "v" :space-evenly false
                (label :class "github-entry-title" :halign "start" :limit-width 43 :text {run.displayTitle})
                (label :class "github-entry-meta" :halign "start" :text {run.repo + " · " + run.workflowName + " · " + run.conclusion})))))
        (box :class "github-section" :orientation "v" :space-evenly false :spacing 5
          (label :class "github-section-title" :halign "start" :text "ISSUES · NEWEST 3")
          (for issue in {github_data.issues}
            (button :class "github-entry issue" :onclick {"${lib.getExe' pkgs.xdg-utils "xdg-open"} " + issue.url}
              (box :orientation "v" :space-evenly false
                (label :class "github-entry-title" :halign "start" :limit-width 43 :text {"#" + issue.number + "  " + issue.title})
                (label :class "github-entry-meta" :halign "start" :text {issue.repo})))))
        (for source in {github_data.errors}
          (label :class "github-error" :halign "start" :text {"query failed · " + source}))
        (label :class "github-updated" :halign "end" :text {github_data.updated_at})))
  '';
}
