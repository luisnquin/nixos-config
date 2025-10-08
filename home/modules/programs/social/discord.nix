{
  services.arrpc.enable = true;

  programs.nixcord = {
    enable = true;
    discord.enable = false;
    vesktop.enable = true;

    config = {
      themeLinks = [
        "https://raw.githubusercontent.com/refact0r/system24/refs/heads/main/theme/system24.theme.css"
      ];
      frameless = true;
      disableMinSize = true;
      plugins = {
        accountPanelServerProfile.enable = true;
        alwaysExpandRoles.enable = true;
        alwaysTrust.enable = true;
        anonymiseFileNames.enable = true;
        betterGifAltText.enable = true;
        betterGifPicker.enable = true;
        betterNotesBox.enable = true;
        betterRoleContext.enable = true;
        betterRoleDot.enable = true;
        betterSessions.enable = false;
        betterSettings.enable = true;
        betterUploadButton.enable = true;
        biggerStreamPreview.enable = true;
        blurNSFW.enable = true;
        callTimer = {
          enable = true;
          format = "human";
        };
        clearURLs.enable = true;
        colorSighted.enable = true;
        consoleJanitor.enable = true;
        consoleShortcuts.enable = true;
        copyEmojiMarkdown.enable = true;
        copyFileContents.enable = true;
        copyUserURLs.enable = true;
        customRPC = {
          enable = false; # TODO
        };
        dearrow.enable = true;
        decor.enable = true;
        disableCallIdle.enable = true;
        dontRoundMyTimestamps.enable = true;
        experiments = {
          enable = true;
          toolbarDevMenu = true;
        };
        f8Break.enable = true;
        fakeProfileThemes.enable = true;
        favoriteEmojiFirst.enable = true;
        favoriteGifSearch.enable = true;
        fixCodeblockGap.enable = true;
        fixImagesQuality.enable = true;
        fixSpotifyEmbeds.enable = true;
        fixYoutubeEmbeds.enable = true;
        forceOwnerCrown.enable = true;
        friendInvites.enable = true;
        friendsSince.enable = true;
        fullSearchContext.enable = true;
        gameActivityToggle.enable = true;
        gifPaste.enable = true;
        greetStickerPicker.enable = true;
        iLoveSpam.enable = true;
        imageLink.enable = true;
        imageZoom = {
          enable = true;
          nearestNeighbour = true;
        };
        implicitRelationships.enable = true;
        invisibleChat.enable = true;
        keepCurrentChannel.enable = true;
        memberCount.enable = true;
        mentionAvatars.enable = true;
        messageClickActions = {
          enable = true;
          enableDoubleClickToEdit = false;
          enableDoubleClickToReply = false;
        };
        messageLatency = {
          enable = true;
          latency = 4;
        };
        messageLinkEmbeds.enable = true;
        messageLogger.enable = false;
        messageTags.enable = true;
        moreCommands.enable = true;
        moreKaomoji.enable = true;
        moreUserTags.enable = true;
        mutualGroupDMs.enable = true;
        newGuildSettings = {
          enable = true;
          messages = "only@Mentions";
          role = false;
        };
        noBlockedMessages.enable = true;
        noDevtoolsWarning.enable = true;
        noF1.enable = true;
        noMosaic.enable = true;
        noOnboardingDelay.enable = true;
        noPendingCount.enable = true;
        noTypingAnimation.enable = true;
        noUnblockToJump.enable = true;
        normalizeMessageLinks.enable = true;
        openInApp.enable = true;
        overrideForumDefaults.enable = true;
        permissionFreeWill.enable = true;
        permissionsViewer.enable = true;
        petpet.enable = true;
        pictureInPicture.enable = true;
        pinDMs.enable = true;
        platformIndicators.enable = true;
        previewMessage.enable = true;
        quickMention.enable = true;
        quickReply.enable = true;
        reactErrorDecoder.enable = true;
        readAllNotificationsButton.enable = true;
        relationshipNotifier.enable = true;
        replaceGoogleSearch.enable = true;
        replyTimestamp.enable = true;
        revealAllSpoilers.enable = true;
        reverseImageSearch.enable = true;
        roleColorEverywhere.enable = true;
        summaries.enable = true;
        sendTimestamps.enable = true;
        serverInfo.enable = true;
        shikiCodeblocks.enable = true;
        showAllMessageButtons.enable = true;
        showConnections.enable = true;
        showHiddenChannels = {
          enable = true;
          showMode = "muted";
        };
        showHiddenThings.enable = true;
        showTimeoutDuration.enable = true;
        silentMessageToggle.enable = true;
        sortFriendRequests.enable = true;
        spotifyControls.enable = true;
        spotifyCrack.enable = true;
        spotifyShareCommands.enable = true;
        startupTimings.enable = true;
        streamerModeOnStream.enable = true;
        themeAttributes.enable = true;
        translate.enable = true;
        typingIndicator.enable = true;
        typingTweaks.enable = true;
        unindent.enable = true;
        unlockedAvatarZoom.enable = true;
        unsuppressEmbeds.enable = true;
        userVoiceShow.enable = true;
        validReply.enable = true;
        validUser.enable = true;
        voiceChatDoubleClick.enable = true;
        vencordToolbox.enable = true;
        viewIcons.enable = true;
        viewRaw.enable = true;
        voiceDownload.enable = true;
        voiceMessages.enable = true;
        volumeBooster.enable = true;
        youtubeAdblock.enable = true;
      };
    };

    vesktopConfig = {
      plugins = {
        webKeybinds.enable = true;
        webRichPresence.enable = true;
        webScreenShareFixes.enable = true;
      };
    };
  };
}
