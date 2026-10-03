# CLI tuios-apps — núcleo de apps AdvPL de primeira classe (Nível C, fase 1).
# Python puro (tomllib stdlib) → build pyproject sem rede (nixpkgs 24.05).
{ lib, python3Packages, git }:

python3Packages.buildPythonApplication {
  pname = "tuios-apps";
  version = "1.0.0";
  src = ../../apps/tuios-apps;
  format = "pyproject";
  nativeBuildInputs = [ python3Packages.setuptools ];
  nativeCheckInputs = [ python3Packages.pytest git ];
  doCheck = true;
  # explícito: não depender do checkPhase default do buildPythonPackage
  checkPhase = ''
    python -m pytest tests -q
  '';
  meta = {
    description = "Apps AdvPL de primeira classe no tuiOS";
    license = lib.licenses.mit;
    mainProgram = "tuios-apps";
    platforms = lib.platforms.linux;
  };
}
