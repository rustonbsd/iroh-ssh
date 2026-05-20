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
  version = "0.2.11";

  src = fetchFromGitHub {
    owner = "rustonbsd";
    repo = "iroh-ssh";
    tag = finalAttrs.version;
    hash = "sha256-0FbezMQcbqhK6bmfP80R3gbGqbkBewprLqzITKwGCSI=";
  };

  cargoHash = "sha256-2NAl4ClPN+OI1EBAkuL/KSiByxU34sBmYDjnRXYuiOY=";

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

