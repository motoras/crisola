{ pkgs, ... }:

{
  # https://devenv.sh/basics/
  env.GREET = "Welcome 🐸";

  # https://devenv.sh/packages/
  packages = [
    pkgs.direnv
    pkgs.git
    pkgs.gcc
    pkgs.helix
    pkgs.just
    pkgs.cargo-nextest
    pkgs.openssh
    pkgs.rustup
    pkgs.yubikey-manager

    pkgs.bandwhich
    pkgs.bat
    pkgs.bmon
    pkgs.curl
    pkgs.commitizen
    pkgs.dua
    pkgs.eza
    pkgs.fd
    pkgs.fzf
    pkgs.hexyl
    pkgs.hyperfine
    pkgs.jq
    pkgs.jless
    pkgs.netcat-gnu
    pkgs.ouch
    pkgs.ripgrep
    pkgs.xclip


    pkgs.btop
    pkgs.macchina
    pkgs.tldr

  ];

  difftastic.enable = true;
  starship.enable = true;
  starship.config.enable = true;
  enterShell = ''
    unset LD_LIBRARY_PATH
    eval "$(direnv hook bash)"
    eval `ssh-agent -s`
    source "ssha.source"
    source "git-aliases.source"
    source "$(fzf-share)/key-bindings.bash"
    source "$(fzf-share)/completion.bash"
    alias j=just
    alias ls=eza
    alias da="direnv allow"
    alias czc="cz c"
    echo $GREET
  '';

  languages.rust.enable = true;

  # https://devenv.sh/languages/
  # languages.nix.enable = true;

  # https://devenv.sh/pre-commit-hooks/
  pre-commit.hooks.nixpkgs-fmt.enable = true;
  pre-commit.hooks.shellcheck.enable = true;
  pre-commit.hooks.typos.enable = true;
  pre-commit.hooks.commitizen.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";

  # See full reference at https://devenv.sh/reference/options/
}
