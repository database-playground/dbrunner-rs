{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";
  env.REDIS_ADDR = "redis://localhost:6379";
  env.REDIS_INTEGRATION_URI = "redis://localhost:6379";

  # https://devenv.sh/packages/
  packages = [
    pkgs.git
    pkgs.cargo-edit
    pkgs.cargo-nextest
    pkgs.protolint
    pkgs.protobuf
    pkgs.grpcui
  ];

  # https://devenv.sh/languages/
  languages.rust.enable = true;
  languages.rust.channel = "nightly";

  services.redis.enable = true;
  # https://devenv.sh/processes/
  # processes.cargo-watch.exec = "cargo-watch";

  # https://devenv.sh/scripts/
  scripts.hello.exec = ''
    echo hello from $GREET
  '';

  enterShell = ''
    hello
    git --version
  '';

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';

  # https://devenv.sh/pre-commit-hooks/
  pre-commit.hooks = {
    shellcheck.enable = true;
    rustfmt.enable = true;
    clippy.enable = true;
  };

  # See full reference at https://devenv.sh/reference/options/
}
