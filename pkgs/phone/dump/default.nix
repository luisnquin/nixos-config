{
  androidenv,
  jdk17_headless,
  runCommand,
}: let
  sdk =
    (androidenv.composeAndroidPackages {
      platformVersions = ["36"];
      buildToolsVersions = ["36.0.0"];
      includeEmulator = false;
      includeSystemImages = false;
      includeNDK = false;
    }).androidsdk;

  root = "${sdk}/libexec/android-sdk";
in
  runCommand "phone-dump.dex" {nativeBuildInputs = [jdk17_headless];} ''
    cp ${./PhoneDump.java} PhoneDump.java
    javac --release 11 -cp ${root}/platforms/android-36/android.jar -d classes PhoneDump.java
    ${root}/build-tools/36.0.0/d8 --min-api 26 --lib ${root}/platforms/android-36/android.jar --output . classes/*.class
    cp classes.dex $out
  ''
