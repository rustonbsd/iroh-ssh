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
  version = "0.2.8";

  src = fetchFromGitHub {
    owner = "rustonbsd";
    repo = "iroh-ssh";
    tag = finalAttrs.version;
    hash = "sha256-hFPM+U88bb9lST1iE9shbjqOzEzC3qhLQAsOxxqv9Pg=";
  };

  cargoHash = "sha256-zsMz7bu6uGWXk1opE9yjPPeRcbspJgCe2RF4U50610w=";

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

