{pkgs, ...}: {
  programs.lazygit = {
    enable = true;
    package = pkgs.lazygit;
    settings = {
      confirmOnQuit = false;
      disableStartupPopups = false;
      git = {
        allBranchesLogCmd = "git log --graph --all --color=always --abbrev-commit --decorate --date=relative  --pretty=medium";
        autoFetch = false;
        autoRefresh = true;
        branchLogCmd = "git log --graph --color=always --abbrev-commit --decorate --date=relative --pretty=medium {{branchName}} --";
        commit = {
          signOff = false;
          verbose = "default";
        };
        diffContextSize = 3;
        disableForcePushing = false;
        log = {
          order = "topo-order";
          showGraph = "when-maximised";
          showWholeGraph = false;
        };
        merging = {
          args = "";
          manualCommit = false;
        };
        overrideGpg = false;
        paging = {
          colorArg = "always";
          useConfig = false;
        };
        parseEmoji = false;
        skipHookPrefix = "WIP";
      };
      gui = {
        border = "single";
        commandLogSize = 8;
        commitLength = {show = true;};
        expandFocusedSidePanel = false;
        language = "en";
        mainPanelSplitMode = "flexible";
        mouseEvents = true;
        scrollHeight = 3;
        scrollPastBottom = true;
        showBottomLine = false;
        showCommandLog = false;
        showFileTree = true;
        showIcons = true;
        showListFooter = true;
        showRandomTip = true;
        sidePanelWidth = 0.3333;
        skipRewordInEditorWarning = false;
        skipStashWarning = false;
        skipDiscardChangeWarning = false;
        splitDiff = "auto";
        theme = {
          activeBorderColor = ["green" "bold"];
          cherryPickedCommitBgColor = ["cyan"];
          cherryPickedCommitFgColor = ["blue"];
          defaultFgColor = ["default"];
          inactiveBorderColor = ["white"];
          optionsTextColor = ["blue"];
          selectedLineBgColor = ["blue"];
          selectedRangeBgColor = ["blue"];
          unstagedChangesColor = ["red"];
        };
        timeFormat = "02/01/06 15:04 MST";
        windowSize = "half";
      };
      keybinding = {
        branches = {
          checkoutBranchByName = "c";
          createPullRequest = "o";
          createTag = "T";
          fastForward = "f";
          fetchRemote = "f";
          forceCheckoutBranch = "F";
          mergeIntoCurrentBranch = "M";
          pushTag = "P";
          rebaseBranch = "r";
          renameBranch = "R";
          setUpstream = "u";
          viewGitFlowOptions = "i";
          viewPullRequestOptions = "O";
        };
        commitFiles = {checkoutCommitFile = "c";};
        commits = {
          amendToCommit = "A";
          checkoutCommit = "<space>";
          cherryPickCopy = "c";
          cherryPickCopyRange = "C";
          copyCommitMessageToClipboard = "<c-y>";
          createFixupCommit = "F";
          markCommitAsFixup = "f";
          moveDownCommit = "<c-j>";
          moveUpCommit = "<c-k>";
          openLogMenu = "<c-l>";
          pasteCommits = "v";
          pickCommit = "p";
          renameCommit = "r";
          renameCommitWithEditor = "R";
          resetCherryPick = "<c-R>";
          revertCommit = "t";
          squashAboveCommits = "S";
          squashDown = "s";
          tagCommit = "T";
          viewBisectOptions = "b";
          viewResetOptions = "g";
        };
        files = {
          amendLastCommit = "A";
          commitChanges = "c";
          commitChangesWithEditor = "C";
          commitChangesWithoutHook = "w";
          fetch = "f";
          ignoreFile = "i";
          openMergeTool = "M";
          openStatusFilter = "<c-b>";
          refreshFiles = "r";
          stashAllChanges = "s";
          toggleStagedAll = "a";
          toggleTreeView = "`";
          viewResetOptions = "D";
          viewStashOptions = "S";
        };
        main = {
          pickBothHunks = "b";
          toggleDragSelect = "v";
          "toggleDragSelect-alt" = "V";
          toggleSelectHunk = "a";
        };
        stash = {
          popStash = "g";
          renameStash = "r";
        };
        status = {
          checkForUpdate = "u";
          recentRepos = "<enter>";
        };
        submodules = {
          bulkMenu = "b";
          init = "i";
          update = "u";
        };
        universal = {
          appendNewline = "<a-enter>";
          confirm = "<enter>";
          "confirm-alt1" = "y";
          copyToClipboard = "<c-o>";
          createPatchOptionsMenu = "<c-p>";
          createRebaseOptionsMenu = "m";
          decreaseContextInDiffView = "{";
          diffingMenu = "W";
          "diffingMenu-alt" = "<c-e>";
          edit = "e";
          executeCustomCommand = ":";
          extrasMenu = "@";
          filteringMenu = "<c-s>";
          goInto = "<enter>";
          gotoBottom = ">";
          gotoTop = "<";
          increaseContextInDiffView = "}";
          jumpToBlock = ["1" "2" "3" "4" "5"];
          new = "n";
          nextBlock = "<right>";
          nextItem = "<down>";
          "nextItem-alt" = "j";
          nextMatch = "n";
          nextPage = ".";
          nextScreenMode = "+";
          nextTab = "]";
          openFile = "o";
          openRecentRepos = "<c-r>";
          optionMenu = null;
          "optionMenu-alt1" = "?";
          prevBlock = "<left>";
          "prevBlock-alt" = "h";
          prevItem = "<up>";
          "prevItem-alt" = "k";
          prevMatch = "N";
          prevPage = ",";
          prevScreenMode = "_";
          prevTab = "[";
          pullFiles = "p";
          pushFiles = "P";
          quit = "q";
          "quit-alt1" = "<c-c>";
          quitWithoutChangingDirectory = "Q";
          redo = "<c-z>";
          refresh = "R";
          remove = "d";
          return = "<esc>";
          "return-alt1" = null;
          scrollDownMain = "<pgdown>";
          "scrollDownMain-alt1" = "J";
          "scrollDownMain-alt2" = "<c-d>";
          scrollLeft = "H";
          scrollRight = "L";
          scrollUpMain = "<pgup>";
          "scrollUpMain-alt1" = "K";
          "scrollUpMain-alt2" = "<c-u>";
          select = "<space>";
          submitEditorText = "<enter>";
          togglePanel = "<tab>";
          toggleWhitespaceInDiffView = "<c-w>";
          undo = "z";
        };
      };
      notARepository = "quit";
      os = {
        editCommand = "";
        editCommandTemplate = "";
        openCommand = "";
      };
      promptToReturnFromSubprocess = true;
      quitOnTopLevelReturn = false;
      refresher = {
        fetchInterval = 30;
        refreshInterval = 5;
      };
      update = {
        days = 14;
        method = "prompt";
      };
    };
  };
}
