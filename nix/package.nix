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
  version = "0.2.12";

  src = fetchFromGitHub {
    owner = "rustonbsd";
    repo = "iroh-ssh";
    tag = finalAttrs.version;
    hash = "sha256-zt5Adq3U+sp0w5+X1xf/vEgQsxQ9G5tZL4+SOCHo5ws=";
  };

  cargoHash = "sha256-NFiOsBYJ1nJCH8E7F7WuL2Xj0/djdnvrJ/0S+9SItpo=";

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

