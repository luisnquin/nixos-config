{
  rofi-network-manager,
  pkgs,
  ...
}: {
  programs = {
    rofi = {
      enable = true;
      plugins = with pkgs; [
        rofi-calc
      ];
    };

    # rofi-radio = {
    #   enable = true;
    #   broadcasters = [
    #     {
    #       name = "More than ever";
    #       url = "https://youtube.com/playlist?list=PL_qr1c1sjwPmIC5OLHmIeq1D6lB_xUPny&si=xBovVspuc-mPS5ze";
    #       shuffle = true;
    #     }
    #     {
    #       name = "Persona ペルソナ and Rainy Mood";
    #       url = "https://youtu.be/NPqIEFG9Klo";
    #     }
    #     {
    #       name = "Refuge Worldwide";
    #       url = "https://streaming.radio.co/s3699c5e49/listen";
    #     }
    #     {
    #       name = "Morro Rock Radio";
    #       url = "https://youtube.com/playlist?list=PLs1H9tARINqejJDpI3O6f-goOGqSsyj3p";
    #       shuffle = true;
    #     }
    #     {
    #       name = "98.7 Body Heat Radio";
    #       url = "https://youtube.com/playlist?list=PLs1H9tARINqdQMb8RGnSlBS5MqoEyyihF";
    #       shuffle = true;
    #     }
    #     {
    #       name = "Radio Vexelstrom";
    #       url = "https://youtube.com/playlist?list=PLZd11OjQTkPWBASMzCtr0OUl1BsHAsYyD";
    #       shuffle = true;
    #     }
    #     {
    #       name = "MGSV: TPP Original Soundtrack \"Music Tape 1\"";
    #       url = "https://youtube.com/playlist?list=PLk-QnEBGoZdJiUE1iB9g_qmQzgA5uPQFj";
    #       shuffle = true;
    #     }
    #     {
    #       name = "MGSV: TPP Original Soundtrack \"Music Tape 2\"";
    #       url = "https://youtube.com/playlist?list=PLixuIKbcCRz3Jkifv71CHquoMAmIH7d1m";
    #       shuffle = true;
    #     }
    #     {
    #       name = "MGSV: TPP Original Soundtrack \"The Lost Tapes\"";
    #       url = "https://youtube.com/playlist?list=PLJCznWvkR1xlDHez8Ph3HTrzpJtssH4ml";
    #       shuffle = true;
    #     }
    #   ];
    # };
  };

  home.packages = [
    rofi-network-manager
  ];

  xdg.configFile = {
    "rofi/rofi-network-manager.conf".text = builtins.readFile ./../../dots/rofi/rofi-network-manager.conf;
    "rofi/rofi-network-manager.rasi".text = builtins.readFile ./../../dots/rofi/rofi-network-manager.rasi;
    "rofi/config.rasi".text = builtins.readFile ./../../dots/rofi/config.rasi;
  };
}
