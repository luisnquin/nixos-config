{pkgs, ...}: {
  programs.lazygit = {
    enable = true;
    package = pkgs.lazygit;
    settings = {
      gui = {
        windowSize = "half";
        scrollHeight = 3;
        scrollPastBottom = true;
        sidePanelWidth = 0.3333;
        expandFocusedSidePanel = false;
        mainPanelSplitMode = "flexible";
        language = "en";
        timeFormat = "02/01/06 15:04 MST";

        theme = {
          activeBorderColor = ["green" "bold"];
          inactiveBorderColor = ["white"];
          optionsTextColor = ["blue"];
          selectedLineBgColor = ["blue"];
          selectedRangeBgColor = ["blue"];
          cherryPickedCommitBgColor = ["cyan"];
          cherryPickedCommitFgColor = ["blue"];
          unstagedChangesColor = ["red"];
          defaultFgColor = ["default"];
        };

        commitLength = {
          show = true;
        };

        mouseEvents = true;
        skipUnstageLineWarning = false;
        skipStashWarning = false;
        showFileTree = true;
        showListFooter = true;
        showRandomTip = true;
        showBottomLine = false;
        showCommandLog = false;
        showIcons = true;
        commandLogSize = 8;
        splitDiff = "auto";
        skipRewordInEditorWarning = false;
        border = "single";
      };

      git = {
        paging = {
          colorArg = "always";
          useConfig = false;
        };

        commit = {
          signOff = false;
          verbose = "default";
        };

        merging = {
          manualCommit = false;
          args = "";
        };

        log = {
          order = "topo-order";
          showGraph = "when-maximised";
          showWholeGraph = false;
        };

        skipHookPrefix = "WIP";
        autoFetch = false;
        autoRefresh = true;
        branchLogCmd = "git log --graph --color=always --abbrev-commit --decorate --date=relative --pretty=medium {{branchName}} --";
        allBranchesLogCmd = "git log --graph --all --color=always --abbrev-commit --decorate --date=relative  --pretty=medium";
        overrideGpg = false;
        disableForcePushing = false;
        parseEmoji = false;
        diffContextSize = 3;
      };

      os = {
        editCommand = "";
        editCommandTemplate = "";
        openCommand = "";
      };

      refresher = {
        refreshInterval = 5;
        fetchInterval = 30;
      };

      update = {
        method = "prompt";
        days = 14;
      };

      confirmOnQuit = false;
      quitOnTopLevelReturn = false;
      disableStartupPopups = false;
      notARepository = "quit";
      promptToReturnFromSubprocess = true;

      keybinding = {
        universal = {
          quit = "q";
          quit-alt1 = "<c-c>";
          return = "<esc>";
          return-alt1 = null;
          quitWithoutChangingDirectory = "Q";
          togglePanel = "<tab>";
          prevItem = "<up>";
          nextItem = "<down>";
          prevItem-alt = "k";
          nextItem-alt = "j";
          prevPage = ",";
          nextPage = ".";
          gotoTop = "<";
          gotoBottom = ">";
          scrollLeft = "H";
          scrollRight = "L";
          prevBlock = "<left>";
          nextBlock = "<right>";
          prevBlock-alt = "h";
          jumpToBlock = ["1" "2" "3" "4" "5"];
          nextMatch = "n";
          prevMatch = "N";
          optionMenu = null; # show help menu
          optionMenu-alt1 = "?"; # show help menu
          select = "<space>";
          goInto = "<enter>";
          openRecentRepos = "<c-r>";
          confirm = "<enter>";
          confirm-alt1 = "y";
          remove = "d";
          new = "n";
          edit = "e";
          openFile = "o";
          scrollUpMain = "<pgup>"; # main panel scroll up
          scrollDownMain = "<pgdown>"; # main panel scroll down
          scrollUpMain-alt1 = "K"; # main panel scroll up
          scrollDownMain-alt1 = "J"; # main panel scroll down
          scrollUpMain-alt2 = "<c-u>"; # main panel scroll up
          scrollDownMain-alt2 = "<c-d>"; # main panel scroll down
          executeCustomCommand = ":";
          createRebaseOptionsMenu = "m";
          pushFiles = "P";
          pullFiles = "p";
          refresh = "R";
          createPatchOptionsMenu = "<c-p>";
          nextTab = "]";
          prevTab = "[";
          nextScreenMode = "+";
          prevScreenMode = "_";
          undo = "z";
          redo = "<c-z>";
          filteringMenu = "<c-s>";
          diffingMenu = "W";
          diffingMenu-alt = "<c-e>"; # deprecated
          copyToClipboard = "<c-o>";
          submitEditorText = "<enter>";
          appendNewline = "<a-enter>";
          extrasMenu = "@";
          toggleWhitespaceInDiffView = "<c-w>";
          increaseContextInDiffView = "}";
          decreaseContextInDiffView = "{";
        };

        status = {
          checkForUpdate = "u";
          recentRepos = "<enter>";
        };

        files = {
          commitChanges = "c";
          commitChangesWithoutHook = "w"; # commit changes without pre-commit hook
          amendLastCommit = "A";
          commitChangesWithEditor = "C";
          ignoreFile = "i";
          refreshFiles = "r";
          stashAllChanges = "s";
          viewStashOptions = "S";
          toggleStagedAll = "a"; # stage/unstage all
          viewResetOptions = "D";
          fetch = "f";
          toggleTreeView = "`";
          openMergeTool = "M";
          openStatusFilter = "<c-b>";
        };

        branches = {
          createPullRequest = "o";
          viewPullRequestOptions = "O";
          checkoutBranchByName = "c";
          forceCheckoutBranch = "F";
          rebaseBranch = "r";
          renameBranch = "R";
          mergeIntoCurrentBranch = "M";
          viewGitFlowOptions = "i";
          fastForward = "f"; # fast-forward this branch from its upstream
          createTag = "T";
          pushTag = "P";
          setUpstream = "u"; # set as upstream of checked-out branch
          fetchRemote = "f";
        };

        commits = {
          squashDown = "s";
          renameCommit = "r";
          renameCommitWithEditor = "R";
          viewResetOptions = "g";
          markCommitAsFixup = "f";
          createFixupCommit = "F"; # create fixup commit for this commit
          squashAboveCommits = "S";
          moveDownCommit = "<c-j>"; # move commit down one
          moveUpCommit = "<c-k>"; # move commit up one
          amendToCommit = "A";
          pickCommit = "p"; # pick commit (when mid-rebase)
          revertCommit = "t";
          cherryPickCopy = "c";
          cherryPickCopyRange = "C";
          pasteCommits = "v";
          tagCommit = "T";
          checkoutCommit = "<space>";
          resetCherryPick = "<c-R>";
          copyCommitMessageToClipboard = "<c-y>";
          openLogMenu = "<c-l>";
          viewBisectOptions = "b";
        };

        stash = {
          popStash = "g";
          renameStash = "r";
        };

        commitFiles.checkoutCommitFile = "c";

        main = {
          toggleDragSelect = "v";
          toggleDragSelect-alt = "V";
          toggleSelectHunk = "a";
          pickBothHunks = "b";
        };

        submodules = {
          init = "i";
          update = "u";
          bulkMenu = "b";
        };
      };
    };
  };
}
