{
  testers,
  tripwire,
}:
testers.runNixOSTest {
  name = "tripwire";

  nodes.machine = {
    imports = [./nixos-modules];

    users.users.alice.isNormalUser = true;

    services.tripwire = {
      enable = true;
      package = tripwire;
      user = "alice";
      decoys = [".git-credentials" ".config/Some Browser/Login Data"];
    };
  };

  testScript = ''
    decoy = "/home/alice/.git-credentials"
    spaced = "/home/alice/.config/Some Browser/Login Data"

    machine.wait_for_unit("tripwire.service")
    machine.succeed(f"test -s '{spaced}'")
    machine.succeed(f"stat {decoy} && ls -la /home/alice")
    machine.fail("journalctl -u tripwire | grep 'opened by'")

    machine.succeed(
        f"su alice -c \"setsid sh -c 'cat {decoy} > /tmp/out 2>&1; touch /tmp/leaked'\" >&2 &"
    )
    machine.wait_until_succeeds("journalctl -u tripwire | grep 'opened by.*cat'")
    machine.succeed("ps -o stat= -C cat | grep T")
    machine.succeed("ps -o stat=,args= -C sh | grep '^T.*leaked'")
    machine.fail("test -e /tmp/leaked")

    machine.succeed("pkill -CONT -u alice")
    machine.wait_until_succeeds("grep 'Operation not permitted' /tmp/out")

    machine.succeed(f"rm {decoy}")
    machine.wait_until_succeeds("journalctl -u tripwire | grep 'removed or altered'")
    machine.wait_until_succeeds(f"test -s {decoy}")
    machine.succeed(f"su alice -c \"setsid cat '{spaced}'\" >&2 &")
    machine.wait_until_succeeds("journalctl -u tripwire | grep 'Login Data opened by'")
    machine.succeed("pkill -KILL -u alice")
  '';
}
