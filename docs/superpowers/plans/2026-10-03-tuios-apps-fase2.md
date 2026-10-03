# tuios-apps Fase 2 (c-tui) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fechar o Nível C para o usuário final: menu dialog sobre os módulos, app-exemplo de fábrica na ISO, cópia de apps pelo instalador, evidência no T2 e guia de documentação (spec R14–R20).

**Architecture:** `menu.py` é uma camada fina que chama `dialog` via `subprocess` (lista de args, sem shell) e reusa `descobrir/buscar/rodar/adicionar` — nunca importa `cli.py` (evita ciclo). O exemplo `ola-tuios` versionado vira derivação Nix e chega ao `/opt/tuios/apps` da live por tmpfiles `C` (cópia só se destino ausente/vazio; man tmpfiles.d verificada), e vai ao destino por `cp -a` no `instalar.sh`.

**Tech Stack:** Python 3.11+ stdlib (tomllib/subprocess), `dialog` (já em systemPackages da ISO e do destino), Nix `runCommand`, bash asserts (mesmos padrões da Fase 1).

## Global Constraints

- Mensagens e docs em **PT-BR**; identificadores técnicos em inglês/abreviações Protheus.
- Exit codes: CLI `0/1/2/3/4`; menu `0` sair/ok · `1` erro geral · `4` dialog ausente (erros de domínio viram `--msgbox` dentro do menu).
- **Zero dependências PyPI** — só stdlib (`tomllib`, `subprocess`, `shutil`, `pathlib`, `os`).
- `subprocess` **sempre** com lista de args (`shell=False`); nunca interpolação.
- `.prw`/`.tlpp` em **CP-1252**; exemplo ASCII puro (sem acentos) é seguro nas duas codificações.
- ProtheusDOC no `.prw` do exemplo, `@author Peder Munksgaard`.
- Commits: `[FEAT|FIX|REF|DOC|STYLE|TEST|CFG|PERF] — descrição curta` PT-BR, **sem** qualquer atribuição de assistente.
- Nix desta máquina: `nix --store /tmp/nix-official ... --print-out-paths --no-link`; caminho físico = prefixo `/tmp/nix-official` no path lógico impresso.
- `test-install` no host: passar `ISO=<arquivo físico da ISO nova>` (symlink `result` é lógico-quebrado).
- Nada de `IIF`, `ConOut` em código cliente (o `ola.prw` do exemplo segue o mesmo template do assert da Fase 1 — `advplc run` standalone, fora do RPO).

---

### Task 1: `menu.py` + subcomando `menu` na CLI (TDD)

**Files:**
- Create: `apps/tuios-apps/tests/test_menu.py`
- Create: `apps/tuios-apps/tuiosapps/menu.py`
- Modify: `apps/tuios-apps/tuiosapps/cli.py` (imports, `_cmd_menu`, subparser, `except DialogAusente`)

**Interfaces:**
- Consumes: `discovery.descobrir/buscar/roots` + `App` (Fase 1), `runner.rodar/AdvplcAusente`, `package.adicionar/ErroPacote`, `manifest.ErroManifesto`.
- Produces: `menu.rodar_menu() -> int`; `menu.DialogAusente(Exception)`; CLI `tuios-apps menu` (exit `0/1/4`). A Task 2 usa o contrato de teste: env `TUIOS_DIALOG` (binário fake), `FAKE_LOG` (1 linha de argv por chamada), `FAKE_QUEUE` (fila `rc|stdout`).

- [ ] **Step 1: Escrever os testes (falham — módulo/subcomando não existem)**

`apps/tuios-apps/tests/test_menu.py` (conteúdo completo):

```python
from pathlib import Path
from types import SimpleNamespace

import pytest

from tuiosapps.cli import main

FAKE = '''#!/usr/bin/env python3
import os, sys, pathlib
log = pathlib.Path(os.environ["FAKE_LOG"])
with log.open("a", encoding="utf-8") as f:
    f.write(" ".join(sys.argv[1:]) + "\\n")
fila = pathlib.Path(os.environ["FAKE_QUEUE"])
linhas = fila.read_text(encoding="utf-8").splitlines() if fila.exists() else []
if not linhas:
    sys.exit(1)
rc, _, saida = linhas[0].partition("|")
fila.write_text("\\n".join(linhas[1:]), encoding="utf-8")
sys.stdout.write(saida)
sys.exit(int(rc))
'''


@pytest.fixture
def roots_tmp(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Path, Path]:
    sistema = tmp_path / "sistema"
    usuario = tmp_path / "usuario"
    sistema.mkdir()
    usuario.mkdir()
    monkeypatch.setenv("TUIOS_APPS_SYSTEM", str(sistema))
    monkeypatch.setenv("TUIOS_APPS_USER", str(usuario))
    return sistema, usuario


def _instalar_app(usuario: Path, app_valido: Path) -> Path:
    destino = usuario / app_valido.name
    destino.mkdir()
    for arquivo in app_valido.iterdir():
        (destino / arquivo.name).write_bytes(arquivo.read_bytes())
    return destino


@pytest.fixture
def fake_dialog(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> SimpleNamespace:
    scripto = tmp_path / "fake-dialog"
    scripto.write_text(FAKE, encoding="utf-8")
    scripto.chmod(0o755)
    log = tmp_path / "dialog.log"
    fila = tmp_path / "dialog.queue"
    log.write_text("", encoding="utf-8")
    fila.write_text("", encoding="utf-8")
    monkeypatch.setenv("TUIOS_DIALOG", str(scripto))
    monkeypatch.setenv("FAKE_LOG", str(log))
    monkeypatch.setenv("FAKE_QUEUE", str(fila))

    def programar(*linhas: str) -> None:
        fila.write_text("\n".join(linhas), encoding="utf-8")

    def chamadas() -> list[str]:
        return [l for l in log.read_text(encoding="utf-8").splitlines() if l]

    return SimpleNamespace(programar=programar, chamadas=chamadas)


def test_menu_dialog_ausente_exit4(monkeypatch: pytest.MonkeyPatch, capsys):
    monkeypatch.setenv("TUIOS_DIALOG", "/nao/existe/dialog")
    assert main(["menu"]) == 4
    assert "dialog nao encontrado" in capsys.readouterr().err


def test_menu_sair_imediato(fake_dialog: SimpleNamespace, capsys):
    fake_dialog.programar("0|sair")
    assert main(["menu"]) == 0
    chamadas = fake_dialog.chamadas()
    assert len(chamadas) == 1
    assert "--menu" in chamadas[0]


def test_menu_listar_mostra_apps(
    fake_dialog: SimpleNamespace, roots_tmp, app_valido: Path
):
    _, usuario = roots_tmp
    _instalar_app(usuario, app_valido)
    fake_dialog.programar("0|listar", "0|voltar", "0|sair")
    assert main(["menu"]) == 0
    chamadas = fake_dialog.chamadas()
    assert len(chamadas) == 3
    assert "ola-tuios" in chamadas[1]  # tela de listagem cita o app


def test_menu_detalhes_msgbox(
    fake_dialog: SimpleNamespace, roots_tmp, app_valido: Path
):
    _, usuario = roots_tmp
    _instalar_app(usuario, app_valido)
    fake_dialog.programar(
        "0|listar", "0|ola-tuios", "0|detalhes", "0|", "0|voltar", "0|sair"
    )
    assert main(["menu"]) == 0
    chamadas = fake_dialog.chamadas()
    assert len(chamadas) == 6
    assert chamadas[3].startswith("--msgbox")
    assert "Primeiro app tuiOS" in chamadas[3]
    assert "entry: ola.prw" in chamadas[3]


def test_menu_rodar_exit_code(
    fake_dialog: SimpleNamespace, roots_tmp, app_valido: Path, tmp_path: Path, monkeypatch
):
    import stat

    _, usuario = roots_tmp
    _instalar_app(usuario, app_valido)
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    fake = bin_dir / "advplc"
    fake.write_text("#!/bin/sh\nexit 5\n", encoding="utf-8")
    fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
    monkeypatch.setenv("PATH", str(bin_dir))

    fake_dialog.programar(
        "0|listar", "0|ola-tuios", "0|rodar", "0|", "0|voltar", "0|sair"
    )
    assert main(["menu"]) == 0
    chamadas = fake_dialog.chamadas()
    assert chamadas[3].startswith("--msgbox")
    assert "exit code: 5" in chamadas[3]


def test_menu_remover_usuario(
    fake_dialog: SimpleNamespace, roots_tmp, app_valido: Path
):
    _, usuario = roots_tmp
    app_dir = _instalar_app(usuario, app_valido)
    fake_dialog.programar(
        "0|listar", "0|ola-tuios", "0|remover", "0|", "0|", "0|sair"
    )
    assert main(["menu"]) == 0
    assert not app_dir.exists()


def test_menu_remover_sistema_bloqueado(
    fake_dialog: SimpleNamespace, roots_tmp, app_valido: Path
):
    sistema, _ = roots_tmp
    sys_dir = _instalar_app(sistema, app_valido)
    fake_dialog.programar(
        "0|listar", "0|ola-tuios", "0|remover", "0|", "0|voltar", "0|sair"
    )
    assert main(["menu"]) == 0
    chamadas = fake_dialog.chamadas()
    assert chamadas[3].startswith("--msgbox")
    assert "--sistema" in chamadas[3]  # dica do comando da CLI
    assert sys_dir.exists()


def test_menu_instalar_fselect(
    fake_dialog: SimpleNamespace, roots_tmp, app_valido: Path, tmp_path: Path
):
    from tuiosapps.package import empacotar

    _, usuario = roots_tmp
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    fake_dialog.programar("0|instalar", f"0|{tarball}", "0|", "0|sair")
    assert main(["menu"]) == 0
    chamadas = fake_dialog.chamadas()
    assert (usuario / "ola-tuios" / "tuios-app.toml").is_file()
    assert chamadas[2].startswith("--msgbox")
    assert "instalado" in chamadas[2]


def test_menu_instalar_erro_msgbox(
    fake_dialog: SimpleNamespace, roots_tmp, tmp_path: Path
):
    _, usuario = roots_tmp
    lixo = tmp_path / "lixo.tar.gz"
    lixo.write_bytes(b"lixo")
    fake_dialog.programar("0|instalar", f"0|{lixo}", "0|", "0|sair")
    assert main(["menu"]) == 0
    chamadas = fake_dialog.chamadas()
    assert chamadas[2].startswith("--msgbox")
    assert "erro" in chamadas[2]
    assert not (usuario / "ola-tuios").exists()
```

- [ ] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_menu.py -q`
Expected: erro de coleção `ModuleNotFoundError: tuiosapps.menu` **e/ou** `SystemExit: 2` (argparse não conhece `menu`).

- [ ] **Step 3: Implementar `menu.py`**

`apps/tuios-apps/tuiosapps/menu.py` (conteúdo completo):

```python
"""Menu TUI (dialog) — camada fina sobre os módulos do tuios-apps (R14, R15)."""
from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

from .discovery import App, buscar, descobrir, roots
from .manifest import ErroManifesto
from .package import ErroPacote, adicionar
from .runner import AdvplcAusente, rodar


class DialogAusente(Exception):
    """Binário dialog fora do PATH/inválido — o CLI converte em exit 4."""


def _resolver_dialog() -> str:
    alvo = os.environ.get("TUIOS_DIALOG", "dialog")
    if os.path.sep in alvo:
        if os.path.isfile(alvo) and os.access(alvo, os.X_OK):
            return alvo
    else:
        achado = shutil.which(alvo)
        if achado:
            return achado
    raise DialogAusente(
        "dialog nao encontrado no PATH (instale dialog ou defina TUIOS_DIALOG)"
    )


def _dialog(alvo: str, args: list[str]) -> tuple[int, str]:
    """Uma invocação do dialog; devolve (rc, stdout)."""
    r = subprocess.run([alvo, *args], capture_output=True, text=True, shell=False)
    return r.returncode, r.stdout.strip()


def _msgbox(alvo: str, texto: str) -> None:
    _dialog(alvo, ["--msgbox", texto, "0", "0"])


def _raiz_usuario() -> Path:
    return next(c for o, c in roots() if o == "usuario")


def _tela_listar(alvo: str) -> None:
    apps = descobrir()
    if not apps:
        _msgbox(alvo, "nenhum app instalado")
        return
    itens: list[str] = []
    for a in apps:
        itens += [a.nome, f"{a.versao} — {a.descricao} [{a.origem}]"]
    itens += ["voltar", "Voltar"]
    rc, escolha = _dialog(
        alvo, ["--stdout", "--menu", "Apps instalados", "0", "0", "0", *itens]
    )
    if rc != 0 or escolha == "voltar":
        return
    app = buscar(escolha)
    if app is None:
        _msgbox(alvo, f"app nao encontrado: {escolha}")
        return
    _tela_app(alvo, app)


def _tela_app(alvo: str, app: App) -> None:
    while True:
        rc, acao = _dialog(
            alvo,
            [
                "--stdout", "--menu", f"{app.nome} {app.versao}", "0", "0", "0",
                "detalhes", "Ver detalhes",
                "rodar", "Rodar agora",
                "remover", "Remover",
                "voltar", "Voltar",
            ],
        )
        if rc != 0 or acao == "voltar":
            return
        if acao == "detalhes":
            _msgbox(
                alvo,
                f"nome: {app.nome}\nversao: {app.versao}\n"
                f"descricao: {app.descricao}\norigem: {app.origem}\n"
                f"entry: {app.entry}\npath: {app.path}",
            )
        elif acao == "rodar":
            try:
                codigo = rodar(app, [])
            except AdvplcAusente as exc:
                _msgbox(alvo, str(exc))
                continue
            _msgbox(alvo, f"exit code: {codigo}")
        elif acao == "remover":
            if app.origem != "usuario":
                _msgbox(
                    alvo,
                    f"app de sistema — remova pela CLI: "
                    f"tuios-apps remover {app.nome} --sistema",
                )
                continue
            rc2, _ = _dialog(alvo, ["--yesno", f"Remover {app.nome}?", "0", "0"])
            if rc2 != 0:
                continue
            try:
                shutil.rmtree(app.path)
            except OSError as exc:
                _msgbox(alvo, f"erro ao remover: {exc}")
                continue
            _msgbox(alvo, f"removido: {app.path}")
            return


def _tela_instalar(alvo: str) -> None:
    rc, caminho = _dialog(
        alvo, ["--stdout", "--fselect", str(Path.home()) + "/", "0", "0"]
    )
    if rc != 0 or not caminho:
        return
    try:
        instalado = adicionar(Path(caminho), _raiz_usuario())
    except (ErroPacote, ErroManifesto) as exc:
        _msgbox(alvo, f"erro: {exc}")
        return
    _msgbox(alvo, f"instalado: {instalado}")


def rodar_menu() -> int:
    """Loop principal do menu; devolve 0 ao sair (erros viram msgbox)."""
    alvo = _resolver_dialog()
    while True:
        rc, escolha = _dialog(
            alvo,
            [
                "--stdout", "--menu",
                "tuios-apps — apps AdvPL de primeira classe",
                "0", "0", "0",
                "listar", "Listar apps instalados",
                "instalar", "Instalar pacote .tar.gz",
                "sair", "Sair",
            ],
        )
        if rc != 0 or escolha == "sair":
            return 0
        if escolha == "listar":
            _tela_listar(alvo)
        elif escolha == "instalar":
            _tela_instalar(alvo)
```

- [ ] **Step 4: Rodar e verificar que ainda falha só na CLI (wiring)**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_menu.py -q`
Expected: ainda falha — `SystemExit: 2`/involução do argparse (subcomando `menu` não existe em `cli.py`).

- [ ] **Step 5: Ligar o subcomando na CLI**

`apps/tuios-apps/tuiosapps/cli.py` — três edits:

1. Junto aos imports do bloco Fase 1, adicionar:

```python
from .menu import DialogAusente, rodar_menu
```

2. Nova função (junto das demais `_cmd_*`):

```python
def _cmd_menu(args: argparse.Namespace) -> int:
    return rodar_menu()
```

3. No parser (após o subparser `remover`), registrar:

```python
    p = sub.add_parser("menu", help="menu interativo (dialog)")
    p.set_defaults(func=_cmd_menu)
```

4. Em `main()`, após o `except AdvplcAusente`:

```python
    except DialogAusente as exc:
        _erro(str(exc))
        return EXIT_DEPENDENCIA
```

- [ ] **Step 6: Rodar e verificar a passagem**

Run: `cd apps/tuios-apps && python3 -m pytest tests -q`
Expected: `10 passed` novo + total da suíte (55 + 9 novos = `64 passed`) — nenhum teste da Fase 1 quebrado.

- [ ] **Step 7: Commit**

```bash
git add apps/tuios-apps
git commit -m "[FEAT] — menu dialog tuios-apps sobre os modulos (R14, R15)"
```

---

### Task 2: Smoke do menu no `apps.assert.sh` (R20)

**Files:**
- Modify: `scripts-assert/apps.assert.sh` (nova seção 10 antes do `APPS-ASSERT-OK`)

**Interfaces:**
- Consumes: contrato `TUIOS_DIALOG`/`FAKE_LOG`/`FAKE_QUEUE` da Task 1; estado da seção 9 (roots com `ola-tuios` usuário 9.9.9 + sistema 1.0.0 → descoberto único `ola-tuios`).

- [ ] **Step 1: Acrescentar a seção 10**

Antes da linha final `echo "APPS-ASSERT-OK"` de `scripts-assert/apps.assert.sh`, inserir:

```bash
# 10. menu com dialog fake (R14/R20) — sem terminal, fila de respostas
FAKE="$W/fake-dialog"
cat > "$FAKE" << 'FDEOF'
#!/usr/bin/env python3
import os, sys, pathlib
log = pathlib.Path(os.environ["FAKE_LOG"])
with log.open("a", encoding="utf-8") as f:
    f.write(" ".join(sys.argv[1:]) + "\n")
fila = pathlib.Path(os.environ["FAKE_QUEUE"])
linhas = fila.read_text(encoding="utf-8").splitlines() if fila.exists() else []
if not linhas:
    sys.exit(1)
rc, _, saida = linhas[0].partition("|")
fila.write_text("\n".join(linhas[1:]), encoding="utf-8")
sys.stdout.write(saida)
sys.exit(int(rc))
FDEOF
chmod +x "$FAKE"
: > "$W/dialog.log"
printf '0|listar\n0|voltar\n0|sair\n' > "$W/dialog.queue"
TUIOS_DIALOG="$FAKE" FAKE_LOG="$W/dialog.log" FAKE_QUEUE="$W/dialog.queue" \
  "$BIN" menu >/dev/null || falha "menu"
grep -q -- "--menu" "$W/dialog.log" || falha "menu nao invocou dialog"
grep -q "ola-tuios" "$W/dialog.log" || falha "menu nao listou ola-tuios"
```

Obs.: o heredoc do assert usa delimitador `FDEOF` (nunca `EOF` — o shell externo interpretaria o EOF interno).

- [ ] **Step 2: Rodar o assert (deve passar — CLI da Task 1 existe)**

Run: `bash scripts-assert/apps.assert.sh`
Expected: `APPS-ASSERT-OK`
Se falhar em `menu`, a fila de respostas não bate com a ordem de invocações — conferir contra `_tela_*` da Task 1.

- [ ] **Step 3: Commit**

```bash
git add scripts-assert/apps.assert.sh
git commit -m "[TEST] — smoke do menu com dialog fake no apps.assert (R20)"
```

---

### Task 3: App-exemplo de fábrica + empacotamento Nix (R16)

**Files:**
- Create: `apps/exemplos/ola-tuios/tuios-app.toml`
- Create: `apps/exemplos/ola-tuios/ola.prw`
- Create: `nixos/apps/exemplos.nix`
- Modify: `flake.nix` (let + packages + specialArgs)
- Modify: `nixos/iso.nix` (arg + tmpfiles)

**Interfaces:**
- Consumes: root `/opt/tuios/apps` (R3 da Fase 1 — `discovery` já o varre); root `/` da live é **tmpfs** (eval verificado: `fileSystems."/".device == "tmpfs"`).
- Produces: derivação `tuiosExemplos` (attr `.#tuiosExemplos`); regra tmpfiles `C /opt/tuios/apps/ola-tuios ...` na live.

- [ ] **Step 1: Criar o app-exemplo**

`apps/exemplos/ola-tuios/tuios-app.toml`:

```toml
nome = "ola-tuios"
versao = "1.0.0"
descricao = "Primeiro app tuiOS"
entry = "ola.prw"
categoria = "exemplo"
autor = "Peder Munksgaard"
```

`apps/exemplos/ola-tuios/ola.prw` (ASCII puro — seguro em CP-1252 e UTF-8):

```
/*/{Protheus.doc} ola-tuios
    App-exemplo de fabrica do tuiOS (Nivel C, F6).
    @author Peder Munksgaard
    @since 03/10/2026
    @version 1.0.0
/*/
User Function Ola()
    ConOut("OLA-TUIOS-OK")
Return .T.
```

- [ ] **Step 2: Validar o manifesto pela CLI da Fase 1**

Run: `apps/tuios-apps/tuios-apps validar apps/exemplos/ola-tuios`
Expected: `OK: ola-tuios 1.0.0 (entry: ola.prw)`

- [ ] **Step 3: Derivação `nixos/apps/exemplos.nix`**

```nix
# App-exemplo de fabrica do Nivel C — copiado para /opt/tuios/apps na live (R16).
# tmpfiles C so copia se o destino nao existe/esta vazio (man tmpfiles.d).
{ runCommand }:

runCommand "tuios-exemplos" { } ''
  mkdir -p $out
  cp -r ${../../apps/exemplos}/. $out/
''
```

- [ ] **Step 4: Build do pacote de exemplos**

Run:
```bash
OUT=$(nix --store /tmp/nix-official build .#tuiosExemplos --print-out-paths --no-link 2>/dev/null | tail -1)
ls "/tmp/nix-official$OUT/ola-tuios/"
```
Expected: `ola.prw  tuios-app.toml`
(obs: antes precisa do `git add` dos arquivos novos — o flake só enxerga rastreados/staged)

- [ ] **Step 5: Wiring no `flake.nix`** (3 pontos, mesmo padrão do `tuiosApps`)

1. No `let`, após a linha `tuiosApps = pkgs.callPackage ./nixos/apps/tuios-apps.nix { };`:

```nix
      # App-exemplo de fabrica do Nivel C (spec Fase 2, R16)
      tuiosExemplos = pkgs.callPackage ./nixos/apps/exemplos.nix { };
```

2. Em `packages.${system}`, após `inherit tuiosApps;`:

```nix
        # App-exemplo de fabrica
        inherit tuiosExemplos;
```

3. Em `specialArgs`, trocar por:

```nix
        specialArgs = { inherit tuios advplc tuiosApps tuiosExemplos; nixpkgsPath = nixpkgs.outPath; nixpkgsSrc = nixpkgs; };
```

- [ ] **Step 6: Wiring no `nixos/iso.nix`** (2 pontos)

1. Args (linha 1):

```nix
{ modulesPath, pkgs, lib, tuios, advplc, tuiosApps, tuiosExemplos, nixpkgsPath, nixpkgsSrc, ... }:
```

2. `systemd.tmpfiles.rules` existente, trocar por:

```nix
  systemd.tmpfiles.rules = [
    "d /var/lib/advpp 0755 root root -"
    # Exemplo de fabrica do Nivel C (R16): copia so se ausente/vazio;
    # apagado por completo, restaura no proximo boot; conteudo do usuario nunca e sobrescrito.
    "d /opt/tuios/apps 0755 root root -"
    "C /opt/tuios/apps/ola-tuios - - - - ${tuiosExemplos}/ola-tuios"
  ];
```

- [ ] **Step 7: Eval de dry-run da ISO**

Run: `nix --store /tmp/nix-official build .#nixosConfigurations.iso.config.system.build.isoImage --dry-run 2>&1 | tail -3`
Expected: lista de `.drv` a construir, **sem** erro de eval/sintaxe/arg.

- [ ] **Step 8: Commit**

```bash
git add apps/exemplos nixos/apps/exemplos.nix flake.nix nixos/iso.nix
git commit -m "[FEAT] — exemplo de fabrica ola-tuios na ISO via tmpfiles (R16)"
```

---

### Task 4: Instalador copia apps ao destino + T2 prova o exemplo (R17, R18)

**Files:**
- Modify: `installer/lib/instalar.sh` (`gerar_config`)
- Modify: `scripts-assert/install.assert.sh` (T2 comando + assert)

**Interfaces:**
- Consumes: exemplo já materializado em `/opt/tuios/apps/ola-tuios` na live (Task 3); `limpar_log`/`enviar`/`esperar` do assert (padrão Fase 1).
- Produze: destino com `/opt/tuios/apps/ola-tuios` real; T2 exige `"nome": "ola-tuios"` + `"origem": "sistema"`.

- [ ] **Step 1: Cópia no `instalar.sh`**

Em `installer/lib/instalar.sh`, função `gerar_config()`, logo **após** a linha
`install -m 644 /etc/tuios-installer/tuios-installer-pkg.nix /mnt/etc/nixos/tuios-installer-pkg.nix`
(e antes de `locale_aplicar`), inserir:

```bash
  # Apps do Nivel C (root de sistema da live) → destino (spec Fase 2, R17)
  if [ -d /opt/tuios/apps ]; then
    mkdir -p /mnt/opt/tuios/apps
    cp -a /opt/tuios/apps/. /mnt/opt/tuios/apps/
  fi
```

- [ ] **Step 2: Trocar o comando e o assert do T2**

Em `scripts-assert/install.assert.sh`:

1. Trocar:

```bash
enviar "systemctl is-active tuios-session; hostname; advplc --version 2>&1 | head -1; tuios-apps listar --json; echo T2-FIM"
```

por (inalterado — já contém `tuios-apps listar --json`; apenas conferir que está assim).

2. Trocar o bloco de assert do `[]` (linhas ~110-111):

```bash
# tuios-apps instalado e respondendo (JSON vazio — exemplos chegam na fase c-tui)
limpar_log | grep -q '^\[\]$' || { echo "FALHA: tuios-apps listar --json nao retornou []"; limpar_log | tail -30; exit 1; }
```

por:

```bash
# tuios-apps instalado com o exemplo de fabrica (R18 — veio da live via cp do instalador)
limpar_log | grep -q '"nome": "ola-tuios"' || { echo "FALHA: ola-tuios nao listado no T2"; limpar_log | tail -30; exit 1; }
limpar_log | grep -q '"origem": "sistema"' || { echo "FALHA: ola-tuios nao veio como sistema"; limpar_log | tail -30; exit 1; }
```

- [ ] **Step 3: Sintaxe dos dois scripts**

Run: `bash -n installer/lib/instalar.sh && bash -n scripts-assert/install.assert.sh && echo SINTAXE-OK`
Expected: `SINTAXE-OK`

- [ ] **Step 4: Commit**

```bash
git add installer/lib/instalar.sh scripts-assert/install.assert.sh
git commit -m "[FEAT] — instalador copia apps de sistema ao destino e T2 prova o exemplo (R17, R18)"
```

---

### Task 5: Guia `docs/tuios-apps.md` + ponte no README (R19)

**Files:**
- Create: `docs/tuios-apps.md`
- Modify: `README.md` (nova seção após "🧠 AdvPP", linha ~96)

**Interfaces:**
- Consumes: interface real da CLI (Fase 1) + menu (Task 1) + exemplo (Task 3).
- Produze: guia canônico do Nível C; README linka para ele.

- [ ] **Step 1: Criar o guia**

`docs/tuios-apps.md` (conteúdo completo):

````markdown
# tuios-apps — apps AdvPL de primeira classe (Nível C)

> Guia completo do gerenciador de apps do tuiOS-Prime.
> Spec: [Fase 1 (c-core)](superpowers/specs/2026-10-03-tuios-apps-fase1-c-core-design.md) ·
> [Fase 2 (c-tui)](superpowers/specs/2026-10-03-tuios-apps-fase2-c-tui-design.md)

## O que é

O **tuios-apps** trata programas AdvPL como **apps nomeados**: um diretório
com um manifesto `tuios-app.toml`. Dá para listar, inspecionar, executar,
empacotar e instalar — pela linha de comando ou por um menu interativo.

## O manifesto `tuios-app.toml`

```toml
nome = "ola-tuios"          # obrigatório — slug: ^[a-z0-9][a-z0-9-]{0,40}$
versao = "1.0.0"            # obrigatório — N.N.N
descricao = "Primeiro app"  # obrigatório — 1..120 caracteres
entry = "ola.prw"           # obrigatório — relativo, existe, .prw/.tlpp
categoria = "exemplo"       # opcional
autor = "Peder Munksgaard"  # opcional
```

Chave desconhecida, entry fora do diretório ou versão fora do padrão →
**exit 2** com a chave apontada.

## CLI

```
tuios-apps listar [--json]              # apps instalados (ordenados)
tuios-apps info <nome>                  # detalhes de um app
tuios-apps rodar <nome> [-- args...]    # executa via advplc run (cwd = app)
tuios-apps validar [dir]                # valida manifesto do diretório
tuios-apps empacotar <dir> [-o saida]   # gera <app>.tar.gz com SHA256SUMS
tuios-apps adicionar <tarball> [--sistema]   # instala (valida tudo antes)
tuios-apps remover <nome> [--sistema]   # remove (sistema exige root)
tuios-apps menu                         # menu interativo (dialog)
tuios-apps --version
```

**Exit codes:** `0` sucesso · `1` erro geral · `2` manifesto/pacote inválido ·
`3` app não encontrado · `4` dependência ausente (`advplc`/`dialog`).

## Menu interativo

`tuios-apps menu` abre um menu com `dialog`:

- **Listar apps** → escolher um app → ver detalhes, rodar ou remover
- **Instalar pacote .tar.gz** → escolhe o arquivo → validação completa
- **Sair**

O menu opera o diretório do **usuário**; operações no root de sistema
(`/opt/tuios/apps`) ficam na CLI com `--sistema` (root). Sem `dialog`
instalado: exit 4. `TUIOS_DIALOG` sobrepõe o binário (testes).

## Roots e sombra

| Root | Caminho |
|------|---------|
| sistema | `/opt/tuios/apps` (exemplos de fábrica; `--sistema` grava aqui) |
| usuário | `${XDG_DATA_HOME:-~/.local/share}/tuios/apps` (padrão) |

Mesmo nome nos dois roots → **o do usuário sombreia o do sistema**.
As envs `TUIOS_APPS_SYSTEM`/`TUIOS_APPS_USER` sobrepõem os caminhos (testes).

## Ciclo de pacote

1. `tuios-apps empacotar minha-pasta` → `nome-1.0.0.tar.gz`
   (prefixo `nome/`, inclui manifesto + `SHA256SUMS`; exclui `.git`,
   `__pycache__`, `*.pyc`)
2. `tuios-apps adicionar nome-1.0.0.tar.gz` → valida em staging:
   gzip → prefixo único → sem `..`/absoluto/symlink externo → checksum →
   manifesto; **só então** instala (falha = nada instalado)
3. `tuios-apps remover nome`

## Exemplo de fábrica

A ISO traz o app **`ola-tuios`** em `/opt/tuios/apps` (copiado no boot via
tmpfiles — some se apagado, conteúdo do usuário nunca é sobrescrito) e o
instalador o leva para o disco destino. Confira com:

```
tuios-apps listar --json
```

## Desenvolvimento

```bash
just test-python   # pytest (suíte unitária)
just test-apps     # assert do ciclo completo em tmpdir
just test-all      # tudo (inclui os asserts de QEMU)
```

Fonte: `apps/tuios-apps/` (Python stdlib puro, zero deps PyPI);
empacotamento Nix em `nixos/apps/`.
````

- [ ] **Step 2: Ponte no README**

Em `README.md`, **após** o final da seção `## 🧠 AdvPP: AdvPL/TLPP embarcado`
(depois da linha `Mais detalhes: [docs/instalacao/advpp-integracao.md](docs/instalacao/advpp-integracao.md).`), inserir:

```markdown

## 📦 Apps AdvPL (Nível C)

O tuiOS também trata programas AdvPL como **apps**: manifesto `tuios-app.toml`,
lista/executa/empacota pela CLI `tuios-apps` ou pelo menu interativo
`tuios-apps menu`. A ISO já traz o exemplo `ola-tuios`.

- Guia completo: [docs/tuios-apps.md](docs/tuios-apps.md)
```

- [ ] **Step 3: Conferir links**

Run: `grep -n "tuios-apps" README.md docs/tuios-apps.md | head`
Expected: ponte README → `docs/tuios-apps.md` presente e guia criado.

- [ ] **Step 4: Commit**

```bash
git add docs/tuios-apps.md README.md
git commit -m "[DOC] — guia tuios-apps.md + ponte no README (R19)"
```

---

### Task 6: Regressão final (ISO nova + test-all + test-install)

**Files:**
- Modify: apenas se um teste falhar (corrigir o bug apontado)

**Interfaces:**
- Consumes: Tasks 1–5.
- Produze: evidência de aceite da Fase 2 (R14–R20).

- [ ] **Step 1: Build da ISO nova (com exemplo)**

Run: `just iso 2>&1 | tail -2`
Expected: store path da ISO impressa no fim (sem erro de eval).

- [ ] **Step 2: `just test-all`**

Run: `just test-all 2>&1 | tail -8`
Expected: `64 passed` (pytest) + `APPS-ASSERT-OK` + bloco `TODOS OS TESTES PASSARAM! ✅`

- [ ] **Step 3: `just test-install` com a ISO nova**

Run:
```bash
ISO="$(ls /tmp/nix-official$(readlink result)/iso/*.iso)" just test-install 2>&1 | tail -6
```
Expected: `INSTAL-TEST-OK (T1 instalação + T2 boot)` — T2 com o assert do
`ola-tuios` (origem `sistema`) verde.
Se falhar: a mensagem `FALHA: ...` aponta o passo — corrigir (ex.: tmpfiles
não copiou → conferir rule; cópia do instalador ausente → `gerar_config`).

- [ ] **Step 4: Commit (apenas se houve correção)**

```bash
git add -A
git commit -m "[FIX] — correções apontadas pela regressão da Fase 2"
```
Sem correções → sem commit.

---

## Self-Review (do próprio plano)

**1. Cobertura da spec (R14–R20):** R14→Task 1 (menu/saída/exit 4) + Task 2 ·
R15→Task 1 (paridade, bloqueio de sistema com dica `--sistema`) · R16→Task 3 ·
R17→Task 4 · R18→Task 4 · R19→Task 5 · R20→Tasks 1, 2, 6. **Sem lacunas.**

**2. Placeholders:** nenhum TBD/TODO; todo passo de código tem bloco completo;
passos de run têm comando e saída esperada.

**3. Consistência de tipos:** `rodar_menu() -> int` (1→CLI/Task 2) ·
`DialogAusente` (1→CLI) · contrato `TUIOS_DIALOG`/`FAKE_LOG`/`FAKE_QUEUE`
fila `rc|stdout` idêntico no pytest (FAKE com `\\n` escapado na fonte Python)
e no assert (heredoc `FDEOF` com `\n` literal) ✓ · ordem de chamadas do
dialog nas filas conferida contra `_tela_*`: principal→listar→app→(detalhes/
rodar/remover)→…→sair ✓ · `tuiosExemplos` (let→packages→specialArgs→iso arg
→tmpfiles) ✓ · contagem esperada 55+9 = 64 pytest ✓.
