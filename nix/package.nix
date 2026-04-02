{
  lib,
  stdenv,
  rustPlatform,
  fetchFromGitHub,
  autoAddDriverRunpath,
  installShellFiles,
  writableTmpDirAsHomeHook,
  versionCheckHook,
  nix-update-script,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "iroh-ssh";
  version = "0.2.10";

  src = fetchFromGitHub {
    owner = "rustonbsd";
    repo = "iroh-ssh";
    tag = finalAttrs.version;
    hash = "sha256-LXLXKrJ2nJzlW8eNhXS9tr9oGp8RlwJPIojqDIdVZf0=";
  };

  cargoHash = "sha256-/cq/rOzrQ4t0qvdaqM3JhRn8IMncx7jWYDjdYmLCYvc=";

  nativeBuildInputs = [
    autoAddDriverRunpath
    installShellFiles
  ];

  doInstallCheck = true;
  nativeInstallCheckInputs = [
    versionCheckHook
    writableTmpDirAsHomeHook
  ];
  versionCheckProgram = "${placeholder "out"}/bin/iroh-ssh";
  versionCheckProgramArg = "version";

  passthru = {
    updateScript = nix-update-script { };
  };

  meta = {
    description = "ssh without ip";
    homepage = "https://github.com/rustonbsd/iroh-ssh";
    license = lib.licenses.mit;
    mainProgram = "iroh-ssh";
  };
})

