{grimblast, ...}: {
  grimblast = grimblast.overrideAttrs (_oldAttrs: rec {
    prePatch = ''
      substituteInPlace ./grimblast --replace '-t 3000' '-t 3000 -i ${./../home/dots/icons/crop.512.png}'
    '';
  });
}
