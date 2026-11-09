# HDiffPatch (hpatchz / hdiffz) — not packaged in nixpkgs.
# Uses the exact same upstream release binary that spt-additions downloads on
# other distros, patched for NixOS with autoPatchelfHook.
{
  lib,
  stdenv,
  fetchzip,
  autoPatchelfHook,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "hdiffpatch";
  version = "5.1.3";

  src = fetchzip {
    url = "https://github.com/sisong/HDiffPatch/releases/download/v${finalAttrs.version}/hdiffpatch_v${finalAttrs.version}_bin_linux64.zip";
    hash = "sha256-Qa9r0qHYPsbk1f7Qsy/PCbs7W4gAxdxZxWlRefGDNLo=";
    stripRoot = false;
  };

  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ stdenv.cc.cc.lib ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    install -Dm755 linux64/hpatchz $out/bin/hpatchz
    install -Dm755 linux64/hdiffz  $out/bin/hdiffz
    runHook postInstall
  '';

  meta = {
    description = "Binary diff/patch tool (hpatchz/hdiffz), used by SPT downpatchers";
    homepage = "https://github.com/sisong/HDiffPatch";
    license = lib.licenses.mit;
    platforms = [ "x86_64-linux" ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
    mainProgram = "hpatchz";
  };
})
