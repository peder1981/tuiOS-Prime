# tuios-apps (Nível C, Fase 1: c-core) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CLI `tuios-apps` — núcleo de "apps AdvPL de primeira classe": manifesto `tuios-app.toml`, discovery sistema/usuário, execução via `advplc`, ciclo de pacote tar.gz seguro, testado e instalado na ISO NixOS.

**Architecture:** Pacote Python 3.11+ puro (stdlib `tomllib`/`tarfile`, zero deps PyPI) com cinco módulos de responsabilidade única (`manifest`, `discovery`, `runner`, `package`, `cli`) sob um entrypoint `tuios-apps`. Empacotado via `buildPythonApplication` (nixpkgs 24.05), injetado na ISO e no sistema instalado pelo mesmo padrão do `advplc` (flake → specialArgs → `tuios-env.nix`). Testes: pytest (unitário) + `apps.assert.sh` (ciclo e2e no repo) + smoke QEMU no `install.assert.sh` T2.

**Tech Stack:** Python 3.11+ (tomllib, argparse, tarfile, subprocess) · pytest · bash · Nix (nixpkgs 24.05, `format = "pyproject"`) · dialog (Fase 2, fora deste plano)

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-10-03-tuios-apps-fase1-c-core-design.md` — R1–R13 são obrigatórios; exit codes exatos: `0` sucesso, `1` erro geral, `2` manifesto/pacote inválido, `3` app não encontrado, `4` dependência ausente.
- **Idioma:** mensagens de erro/ajuda em PT-BR; mensagens de commit em PT-BR no formato `[FEAT|FIX|REF|DOC|STYLE|TEST|CFG|PERF] — descrição curta`, **sem** qualquer atribuição de assistente.
- **Python:** ≥ 3.11 (tomllib stdlib); **zero dependências PyPI** (pytest é apenas ferramenta de teste, nunca requisito de runtime).
- **Segurança:** `subprocess` **nunca** com `shell=True`; extração de tar rejeita `..`, absolutos e symlinks para fora; `--sistema` exige `os.geteuid() == 0`.
- **Nix:** nixpkgs 24.05 → usar `format = "pyproject"` + `nativeBuildInputs` (attr `build-system` NÃO existe nesta versão). Nesta máquina o nix exige `nix --store /tmp/nix-official` (o Justfile já auto-detecta; comandos soltos do plano usam o flag explícito).
- **Encoding:** arquivos `.py`/`.toml`/`.sh` em UTF-8 (a regra CP-1252 vale só para fontes AdvPL/TLPP).
- **Regressão:** `just test-all` e `just test-install` têm de continuar verdes ao final.

## File Structure

```
apps/tuios-apps/                # NOVO — src do pacote
├── tuios-apps                  # entrypoint executável (dev/local; o Nix gera outro igual via console_scripts)
├── pyproject.toml              # setuptools, [project.scripts] → tuiosapps.cli:main
├── tuiosapps/
│   ├── __init__.py             # versão do pacote
│   ├── manifest.py             # parse TOML + validação estrita (R1, R2)
│   ├── discovery.py            # varredura de roots + modelo App (R3–R5)
│   ├── runner.py               # subprocess advplc (R7, R8)
│   ├── package.py              # empacotar/adicionar/remover + checksum (R9–R11)
│   └── cli.py                  # argparse + exit codes + --json (R6, R12, R13)
└── tests/
    ├── conftest.py             # fixtures: app_valido, apps_root (tmp)
    ├── test_manifest.py
    ├── test_discovery.py
    ├── test_runner.py
    ├── test_package.py
    └── test_cli.py
nixos/apps/tuios-apps.nix       # NOVO — buildPythonApplication
scripts-assert/apps.assert.sh   # NOVO — assert e2e do ciclo (APPS-ASSERT-OK)
Justfile                        # MODIFICAR: test-python, test-apps, test-all
.github/workflows/build.yml     # MODIFICAR: pip install pytest antes de just test-all
scripts-assert/install.assert.sh# MODIFICAR: smoke tuios-apps no T2
flake.nix                       # MODIFICAR: tuiosApps no let/packages/specialArgs
nixos/iso.nix                   # MODIFICAR: arg + systemPackages + tuios-env.nix
nixos/instalado/configuration.nix # MODIFICAR: env.tuiosApps em systemPackages
```

**Contratos entre unidades (bloco Consumes/Produces de cada task):**

| Módulo | Produz (assinaturas exatas) |
|--------|----------------------------|
| `manifest` | `MANIFEST_FILE: str = "tuios-app.toml"` · `class ErroManifesto(Exception)` · `Manifest` (frozen dataclass: `nome:str, versao:str, descricao:str, entry:str, categoria:str\|None, autor:str\|None, path:Path`) · `carregar_manifesto(dir_app: Path) -> Manifest` |
| `discovery` | `App` (frozen dataclass: `nome:str, versao:str, descricao:str, categoria:str\|None, autor:str\|None, origem:str, path:Path, entry:str`) · `roots() -> list[tuple[str, Path]]` · `descobrir(avisar=_aviso_padrao) -> list[App]` · `buscar(nome, avisar=...) -> App \| None` |
| `runner` | `class AdvplcAusente(Exception)` · `rodar(app: App, args: list[str]) -> int` |
| `package` | `class ErroPacote(Exception)` · `empacotar(dir_app: Path, saida: Path \| None = None) -> Path` · `adicionar(tarball: Path, destino: Path) -> Path` · `verificar_checksum(dir_app: Path) -> None` |
| `cli` | `main(argv: list[str] \| None = None) -> int` |

---

### Task 1: Scaffold do pacote Python

**Files:**
- Create: `apps/tuios-apps/pyproject.toml`
- Create: `apps/tuios-apps/tuios-apps`
- Create: `apps/tuios-apps/tuiosapps/__init__.py`
- Create: `apps/tuios-apps/tests/test_smoke.py`
- Create: `apps/tuios-apps/tests/conftest.py`

**Interfaces:**
- Consumes: (nada — scaffold)
- Produces: pacote importável `tuiosapps`; entrypoint `apps/tuios-apps/tuios-apps`; comando de teste padrão **`cd apps/tuios-apps && python3 -m pytest tests -q`** (usado em todas as tasks seguintes)

- [X] **Step 1: Escrever o teste de smoke (falha — pacote não existe)**

`apps/tuios-apps/tests/test_smoke.py`:
```python
import tuiosapps


def test_pacote_importavel():
    assert tuiosapps.__version__
```

- [X] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests -q`
Expected: FAIL com `ModuleNotFoundError: No module named 'tuiosapps'`

- [X] **Step 3: Implementar o scaffold**

`apps/tuios-apps/tuiosapps/__init__.py`:
```python
"""tuios-apps — apps AdvPL de primeira classe no tuiOS (Nível C, fase 1)."""

__version__ = "1.0.0"
```

`apps/tuios-apps/pyproject.toml`:
```toml
[build-system]
requires = ["setuptools>=61"]
build-backend = "setuptools.build_meta"

[project]
name = "tuios-apps"
version = "1.0.0"
description = "Apps AdvPL de primeira classe no tuiOS"
requires-python = ">=3.11"

[project.scripts]
tuios-apps = "tuiosapps.cli:main"

[tool.setuptools]
packages = ["tuiosapps"]
```

`apps/tuios-apps/tuios-apps` (depois: `chmod +x`):
```python
#!/usr/bin/env python3
"""Entrypoint local (dev/CI): importa o pacote que mora ao lado."""
import sys

from tuiosapps.cli import main

if __name__ == "__main__":
    sys.exit(main())
```

`apps/tuios-apps/tests/conftest.py`:
```python
"""Fixtures compartilhadas dos testes do tuios-apps."""
from pathlib import Path

import pytest

MANIFESTO_EXEMPLO = """\
nome = "ola-tuios"
versao = "1.0.0"
descricao = "Primeiro app tuiOS"
entry = "ola.prw"
categoria = "exemplo"
"""

PRW_EXEMPLO = """\
User Function Ola()
    ConOut("OLA-TUIOS-OK")
Return .T.
"""


@pytest.fixture
def app_valido(tmp_path: Path) -> Path:
    """Diretório de app com manifesto válido e entry existente."""
    d = tmp_path / "ola-tuios"
    d.mkdir()
    (d / "tuios-app.toml").write_text(MANIFESTO_EXEMPLO, encoding="utf-8")
    (d / "ola.prw").write_text(PRW_EXEMPLO, encoding="utf-8")
    return d
```

- [X] **Step 4: Rodar e verificar a passagem**

Run: `cd apps/tuios-apps && python3 -m pytest tests -q`
Expected: `1 passed`

- [X] **Step 5: Commit**

```bash
git add apps/tuios-apps
git commit -m "[TEST] — scaffold do pacote tuios-apps (pyproject, entrypoint, smoke)"
```

---

### Task 2: `manifest.py` — parse e validação estrita (R1, R2)

**Files:**
- Create: `apps/tuios-apps/tests/test_manifest.py`
- Create: `apps/tuios-apps/tuiosapps/manifest.py`

**Interfaces:**
- Consumes: scaffold Task 1
- Produces: `MANIFEST_FILE = "tuios-app.toml"`; `ErroManifesto`; `Manifest(nome, versao, descricao, entry, categoria, autor, path)`; `carregar_manifesto(dir_app: Path) -> Manifest`

- [X] **Step 1: Escrever os testes (falham — módulo não existe)**

`apps/tuios-apps/tests/test_manifest.py`:
```python
from pathlib import Path

import pytest

from tuiosapps.manifest import ErroManifesto, carregar_manifesto


def _gravar(dir_app: Path, texto: str) -> Path:
    dir_app.mkdir(parents=True, exist_ok=True)
    (dir_app / "tuios-app.toml").write_text(texto, encoding="utf-8")
    return dir_app


def test_manifesto_valido(app_valido: Path):
    m = carregar_manifesto(app_valido)
    assert m.nome == "ola-tuios"
    assert m.versao == "1.0.0"
    assert m.entry == "ola.prw"
    assert m.categoria == "exemplo"
    assert m.autor is None
    assert m.path == app_valido


def test_sem_manifesto_erro(tmp_path: Path):
    with pytest.raises(ErroManifesto, match="manifesto nao encontrado"):
        carregar_manifesto(tmp_path)


def test_toml_invalido(tmp_path: Path):
    _gravar(tmp_path, "nome = [quebrado")
    with pytest.raises(ErroManifesto, match="TOML invalido"):
        carregar_manifesto(tmp_path)


def test_campo_obrigatorio_ausente(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('descricao = "Primeiro app tuiOS"\n', ""), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="obrigatorio ausente: descricao"):
        carregar_manifesto(app_valido)


def test_chave_desconhecida_erro(app_valido: Path):
    with app_valido.joinpath("tuios-app.toml").open("a", encoding="utf-8") as f:
        f.write('descricao_extra = "typo"\n')  # bare-key ASCII: TOML rejeita ç
    with pytest.raises(ErroManifesto, match="chave desconhecida"):
        carregar_manifesto(app_valido)


def test_nome_fora_do_slug(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('nome = "ola-tuios"', 'nome = "Meu App!"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="nome"):
        carregar_manifesto(app_valido)


def test_versao_nao_semver(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('versao = "1.0.0"', 'versao = "1.0"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="versao"):
        carregar_manifesto(app_valido)


def test_descricao_grande_demais(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('descricao = "Primeiro app tuiOS"', f'descricao = "{"x" * 121}"'),
        encoding="utf-8",
    )
    with pytest.raises(ErroManifesto, match="descricao"):
        carregar_manifesto(app_valido)


def test_entry_inexistente(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('entry = "ola.prw"', 'entry = "sumiu.prw"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="entry"):
        carregar_manifesto(app_valido)


def test_entry_absoluto_rejeitado(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('entry = "ola.prw"', 'entry = "/etc/passwd"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="entry"):
        carregar_manifesto(app_valido)


def test_entry_extensao_errada(app_valido: Path):
    (app_valido / "nota.txt").write_text("x", encoding="utf-8")
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('entry = "ola.prw"', 'entry = "nota.txt"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="entry"):
        carregar_manifesto(app_valido)


def test_campo_nao_texto(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('versao = "1.0.0"', "versao = 1"), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="versao"):
        carregar_manifesto(app_valido)
```

- [X] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_manifest.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tuiosapps.manifest'`

- [X] **Step 3: Implementar `manifest.py`**

`apps/tuios-apps/tuiosapps/manifest.py`:
```python
"""Manifesto tuios-app.toml: parse TOML + validação estrita (R1, R2)."""
from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

MANIFEST_FILE = "tuios-app.toml"
EXTENSOES_VALIDAS = (".prw", ".tlpp")
SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9-]{0,40}$")
VERSAO_RE = re.compile(r"^\d+\.\d+\.\d+$")

CHAVES_OBRIGATORIAS = ("nome", "versao", "descricao", "entry")
CHAVES_OPCIONAIS = ("categoria", "autor")
CHAVES_CONHECIDAS = CHAVES_OBRIGATORIAS + CHAVES_OPCIONAIS


class ErroManifesto(Exception):
    """Manifesto inválido — o CLI converte em exit 2."""


@dataclass(frozen=True)
class Manifest:
    nome: str
    versao: str
    descricao: str
    entry: str
    categoria: str | None
    autor: str | None
    path: Path


def carregar_manifesto(dir_app: Path) -> Manifest:
    """Lê e valida o manifesto do diretório; levanta ErroManifesto em qualquer violação."""
    arquivo = dir_app / MANIFEST_FILE
    if not arquivo.is_file():
        raise ErroManifesto(f"manifesto nao encontrado: {arquivo}")
    try:
        dados = tomllib.loads(arquivo.read_text(encoding="utf-8"))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as exc:
        raise ErroManifesto(f"TOML invalido em {arquivo}: {exc}") from exc

    desconhecidas = [k for k in dados if k not in CHAVES_CONHECIDAS]
    if desconhecidas:
        raise ErroManifesto(
            f"chave desconhecida: {desconhecidas[0]!r} "
            f"(permitidas: {', '.join(CHAVES_CONHECIDAS)})"
        )
    ausentes = [k for k in CHAVES_OBRIGATORIAS if k not in dados]
    if ausentes:
        raise ErroManifesto(f"campo obrigatorio ausente: {ausentes[0]}")

    for campo in CHAVES_CONHECIDAS:
        valor = dados.get(campo)
        if valor is not None and not isinstance(valor, str):
            raise ErroManifesto(f"campo {campo} deve ser texto")

    if not SLUG_RE.match(dados["nome"]):
        raise ErroManifesto(
            f"nome invalido: {dados['nome']!r} (esperado slug ^[a-z0-9][a-z0-9-]{{0,40}}$)"
        )
    if not VERSAO_RE.match(dados["versao"]):
        raise ErroManifesto(f"versao invalida: {dados['versao']!r} (esperado N.N.N)")
    descricao = dados["descricao"].strip()
    if not 1 <= len(descricao) <= 120:
        raise ErroManifesto("descricao deve ter de 1 a 120 caracteres")

    entry = dados["entry"]
    caminho_entry = PurePosixPath(entry)
    if caminho_entry.is_absolute() or ".." in caminho_entry.parts:
        raise ErroManifesto(f"entry deve ser caminho relativo dentro do app: {entry!r}")
    if not entry.endswith(EXTENSOES_VALIDAS):
        raise ErroManifesto(
            f"entry deve terminar em {' ou '.join(EXTENSOES_VALIDAS)}: {entry!r}"
        )
    if not (dir_app / entry).is_file():
        raise ErroManifesto(f"entry nao existe no app: {entry!r}")

    return Manifest(
        nome=dados["nome"],
        versao=dados["versao"],
        descricao=descricao,
        entry=entry,
        categoria=dados.get("categoria"),
        autor=dados.get("autor"),
        path=dir_app,
    )
```

- [X] **Step 4: Rodar e verificar a passagem**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_manifest.py -q`
Expected: `13 passed` (1 smoke + 12 de manifest)

- [X] **Step 5: Commit**

```bash
git add apps/tuios-apps
git commit -m "[FEAT] — manifest tuios-app.toml com validacao estrita (R1, R2)"
```

---

### Task 3: `discovery.py` — varredura de roots (R3–R5)

**Files:**
- Create: `apps/tuios-apps/tests/test_discovery.py`
- Create: `apps/tuios-apps/tuiosapps/discovery.py`

**Interfaces:**
- Consumes: `carregar_manifesto`, `ErroManifesto`, `Manifest` (Task 2)
- Produces: `App(nome, versao, descricao, categoria, autor, origem, path, entry)`; `roots() -> list[tuple[str, Path]]`; `descobrir(avisar=...) -> list[App]`; `buscar(nome, avisar=...) -> App | None`

- [X] **Step 1: Escrever os testes (falham)**

`apps/tuios-apps/tests/test_discovery.py`:
```python
from pathlib import Path

import pytest

from tuiosapps.discovery import buscar, descobrir, roots


@pytest.fixture
def roots_tmp(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Path, Path]:
    sistema = tmp_path / "sistema"
    usuario = tmp_path / "usuario"
    sistema.mkdir()
    usuario.mkdir()
    monkeypatch.setenv("TUIOS_APPS_SYSTEM", str(sistema))
    monkeypatch.setenv("TUIOS_APPS_USER", str(usuario))
    return sistema, usuario


def _copiar_app(origem: Path, destino_root: Path) -> Path:
    destino = destino_root / origem.name
    destino.mkdir()
    for arquivo in origem.iterdir():
        (destino / arquivo.name).write_bytes(arquivo.read_bytes())
    return destino


def _silencio(_msg: str) -> None:
    pass


def test_roots_por_env(roots_tmp):
    sistema, usuario = roots_tmp
    assert roots() == [("sistema", sistema), ("usuario", usuario)]


def test_descobre_app_do_usuario(roots_tmp, app_valido: Path):
    _, usuario = roots_tmp
    _copiar_app(app_valido, usuario)
    apps = descobrir(avisar=_silencio)
    assert [a.nome for a in apps] == ["ola-tuios"]
    assert apps[0].origem == "usuario"
    assert apps[0].entry == "ola.prw"


def test_descobre_app_do_sistema(roots_tmp, app_valido: Path):
    sistema, _ = roots_tmp
    _copiar_app(app_valido, sistema)
    apps = descobrir(avisar=_silencio)
    assert apps[0].origem == "sistema"


def test_pasta_sem_manifesto_pula_com_aviso(roots_tmp, capsys):
    _, usuario = roots_tmp
    (usuario / "lixo").mkdir()
    apps = descobrir()
    assert apps == []
    assert "pulada" in capsys.readouterr().err


def test_manifesto_invalido_pula_com_aviso(roots_tmp, capsys):
    _, usuario = roots_tmp
    (usuario / "quebrado").mkdir()
    (usuario / "quebrado" / "tuios-app.toml").write_text("nome = [x", encoding="utf-8")
    apps = descobrir()
    assert apps == []
    assert "app pulado" in capsys.readouterr().err


def test_usuario_sombrea_sistema(roots_tmp, app_valido: Path):
    sistema, usuario = roots_tmp
    _copiar_app(app_valido, sistema)
    novo = _copiar_app(app_valido, usuario)
    (novo / "tuios-app.toml").write_text(
        (novo / "tuios-app.toml").read_text(encoding="utf-8").replace("1.0.0", "2.0.0"),
        encoding="utf-8",
    )
    apps = descobrir(avisar=_silencio)
    assert len(apps) == 1
    assert apps[0].origem == "usuario"
    assert apps[0].versao == "2.0.0"


def test_nome_duplicado_na_mesma_raiz_mantem_o_primeiro(roots_tmp, app_valido: Path):
    sistema, _ = roots_tmp
    _copiar_app(app_valido, sistema)
    # segunda pasta com o MESMO nome de app (manifesto nome = "ola-tuios")
    segunda = sistema / "alias"
    segunda.mkdir()
    for arquivo in app_valido.iterdir():
        (segunda / arquivo.name).write_bytes(arquivo.read_bytes())
    avisos: list[str] = []
    apps = descobrir(avisar=avisos.append)
    assert len(apps) == 1
    assert any("duplicado" in a for a in avisos)


def test_ordenado_por_nome(roots_tmp, app_valido: Path):
    _, usuario = roots_tmp
    _copiar_app(app_valido, usuario)
    outro = usuario / "ze-app"
    outro.mkdir()
    (outro / "tuios-app.toml").write_text(
        'nome = "ze-app"\nversao = "0.1.0"\ndescricao = "Z"\nentry = "z.prw"\n',
        encoding="utf-8",
    )
    (outro / "z.prw").write_text("Return", encoding="utf-8")
    assert [a.nome for a in descobrir(avisar=_silencio)] == ["ola-tuios", "ze-app"]


def test_raiz_inexistente_ignorada(monkeypatch):
    monkeypatch.setenv("TUIOS_APPS_SYSTEM", "/nao/existe/a")
    monkeypatch.setenv("TUIOS_APPS_USER", "/nao/existe/b")
    assert descobrir(avisar=_silencio) == []


def test_buscar_encontra_e_retorna_none(roots_tmp, app_valido: Path):
    _, usuario = roots_tmp
    _copiar_app(app_valido, usuario)
    assert buscar("ola-tuios", avisar=_silencio) is not None
    assert buscar("nao-existe", avisar=_silencio) is None
```

Obs.: a fixture `roots_tmp` zera os roots via env — os testes nunca tocam `/opt` ou `$HOME`.

- [X] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_discovery.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tuiosapps.discovery'`

- [X] **Step 3: Implementar `discovery.py`**

`apps/tuios-apps/tuiosapps/discovery.py`:
```python
"""Discovery: varre os roots de apps (sistema/usuário) e devolve o índice (R3–R5)."""
from __future__ import annotations

import os
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from .manifest import MANIFEST_FILE, ErroManifesto, carregar_manifesto


@dataclass(frozen=True)
class App:
    nome: str
    versao: str
    descricao: str
    categoria: str | None
    autor: str | None
    origem: str  # "sistema" | "usuario"
    path: Path
    entry: str


def _aviso_padrao(msg: str) -> None:
    print(f"tuios-apps: aviso: {msg}", file=sys.stderr)


def roots() -> list[tuple[str, Path]]:
    """(origem, caminho) dos roots, na ordem de precedência: sistema, usuário.

    TUIOS_APPS_SYSTEM / TUIOS_APPS_USER sobrescrevem (testes usam tmpdir).
    """
    sistema = Path(os.environ.get("TUIOS_APPS_SYSTEM", "/opt/tuios/apps"))
    if "TUIOS_APPS_USER" in os.environ:
        usuario = Path(os.environ["TUIOS_APPS_USER"])
    else:
        xdg = os.environ.get("XDG_DATA_HOME")
        base = Path(xdg) if xdg else Path.home() / ".local" / "share"
        usuario = base / "tuios" / "apps"
    return [("sistema", sistema), ("usuario", usuario)]


def descobrir(avisar: Callable[[str], None] = _aviso_padrao) -> list[App]:
    """Índice de apps ordenado por nome. Manifesto quebrado nunca derruba a varredura."""
    por_nome: dict[str, App] = {}
    for origem, root in roots():
        if not root.is_dir():
            continue
        for filho in sorted(root.iterdir()):
            if not filho.is_dir():
                continue
            if not (filho / MANIFEST_FILE).is_file():
                avisar(f"pasta sem {MANIFEST_FILE}: {filho} (pulada)")
                continue
            try:
                m = carregar_manifesto(filho)
            except ErroManifesto as exc:
                avisar(f"{exc} (app pulado)")
                continue
            app = App(
                nome=m.nome,
                versao=m.versao,
                descricao=m.descricao,
                categoria=m.categoria,
                autor=m.autor,
                origem=origem,
                path=filho,
                entry=m.entry,
            )
            existente = por_nome.get(app.nome)
            if existente is not None and existente.origem == app.origem:
                avisar(
                    f"nome duplicado {app.nome!r} em {root}; "
                    f"mantendo {existente.path}"
                )
                continue
            por_nome[app.nome] = app  # usuário (varrido depois) sombreia o sistema
    return sorted(por_nome.values(), key=lambda a: a.nome)


def buscar(nome: str, avisar: Callable[[str], None] = _aviso_padrao) -> App | None:
    """App pelo nome, ou None se não existir."""
    for app in descobrir(avisar=avisar):
        if app.nome == nome:
            return app
    return None
```

- [X] **Step 4: Rodar e verificar a passagem**

Run: `cd apps/tuios-apps && python3 -m pytest tests -q`
Expected: todos passam (smoke + manifest + discovery)

- [X] **Step 5: Commit**

```bash
git add apps/tuios-apps
git commit -m "[FEAT] — discovery de apps em roots sistema/usuario (R3-R5)"
```

---

### Task 4: `runner.py` — execução via advplc (R7, R8)

**Files:**
- Create: `apps/tuios-apps/tests/test_runner.py`
- Create: `apps/tuios-apps/tuiosapps/runner.py`

**Interfaces:**
- Consumes: `App` (Task 3)
- Produces: `AdvplcAusente`; `rodar(app: App, args: list[str]) -> int` (cwd = `app.path`; comando `advplc run <entry> <args...>`; retorna o exit code do filho)

- [X] **Step 1: Escrever os testes (falham)**

`apps/tuios-apps/tests/test_runner.py`:
```python
import os
import stat
from pathlib import Path

import pytest

from tuiosapps.discovery import App
from tuiosapps.runner import AdvplcAusente, rodar


@pytest.fixture
def app(tmp_path: Path) -> App:
    d = tmp_path / "ola-tuios"
    d.mkdir()
    (d / "ola.prw").write_text("Return", encoding="utf-8")
    return App(
        nome="ola-tuios",
        versao="1.0.0",
        descricao="Primeiro app tuiOS",
        categoria="exemplo",
        autor=None,
        origem="usuario",
        path=d,
        entry="ola.prw",
    )


def test_advplc_ausente_exit4(app: App, monkeypatch: pytest.MonkeyPatch, tmp_path: Path):
    vazio = tmp_path / "path-vazio"
    vazio.mkdir()
    monkeypatch.setenv("PATH", str(vazio))
    with pytest.raises(AdvplcAusente):
        rodar(app, [])


def test_executa_com_cwd_e_argumentos(app: App, monkeypatch: pytest.MonkeyPatch, tmp_path: Path):
    log = tmp_path / "log.txt"
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    fake = bin_dir / "advplc"
    fake.write_text(
        f"""#!/bin/sh
pwd > "{log}"
echo "$@" >> "{log}"
exit 7
""",
        encoding="utf-8",
    )
    fake.chmod(fake.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    monkeypatch.setenv("PATH", str(bin_dir))

    rc = rodar(app, ["--x", "1"])

    assert rc == 7  # exit code do filho propagado
    conteudo = log.read_text(encoding="utf-8").splitlines()
    assert conteudo[0] == str(app.path)  # cwd = dir do app (R7)
    assert conteudo[1] == "run ola.prw --x 1"  # advplc run <entry> + args


def test_nao_usa_shell(app: App, monkeypatch: pytest.MonkeyPatch, tmp_path: Path):
    # argumento com metacaracteres chega literal ao filho (sem shell)
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    fake = bin_dir / "advplc"
    fake.write_text(
        """#!/bin/sh
printf '%s\n' "$@" > "$OUT"
""",
        encoding="utf-8",
    )
    fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
    out = tmp_path / "out.txt"
    monkeypatch.setenv("PATH", str(bin_dir))
    monkeypatch.setenv("OUT", str(out))

    rodar(app, ["; rm -rf /"])

    # argumento chega literal ao filho, sem shell para interpretar
    assert out.read_text(encoding="utf-8").splitlines() == [
        "run",
        "ola.prw",
        "; rm -rf /",
    ]
```

- [X] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_runner.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tuiosapps.runner'`

- [X] **Step 3: Implementar `runner.py`**

`apps/tuios-apps/tuiosapps/runner.py`:
```python
"""Execução de apps via advplc — subprocess sem shell (R7, R8)."""
from __future__ import annotations

import shutil
import subprocess

from .discovery import App


class AdvplcAusente(Exception):
    """advplc fora do PATH — o CLI converte em exit 4."""


def rodar(app: App, args: list[str]) -> int:
    """Roda `advplc run <entry>` com cwd no dir do app; devolve o exit code."""
    if shutil.which("advplc") is None:
        raise AdvplcAusente(
            "advplc nao encontrado no PATH (instale o AdvPP / tuios-apps completo)"
        )
    cmd = ["advplc", "run", app.entry, *args]
    return subprocess.run(cmd, cwd=app.path, shell=False).returncode
```

- [X] **Step 4: Rodar e verificar a passagem**

Run: `cd apps/tuios-apps && python3 -m pytest tests -q`
Expected: todos passam

- [X] **Step 5: Commit**

```bash
git add apps/tuios-apps
git commit -m "[FEAT] — runner advplc com cwd do app e exit code propagado (R7, R8)"
```

---

### Task 5: `package.py` — empacotar/adicionar/remover com checksum (R9–R11)

**Files:**
- Create: `apps/tuios-apps/tests/test_package.py`
- Create: `apps/tuios-apps/tuiosapps/package.py`

**Interfaces:**
- Consumes: `carregar_manifesto`, `ErroManifesto`, `MANIFEST_FILE` (Task 2)
- Produces: `ErroPacote`; `empacotar(dir_app: Path, saida: Path | None = None) -> Path`; `adicionar(tarball: Path, destino: Path) -> Path`; `verificar_checksum(dir_app: Path) -> None`

- [X] **Step 1: Escrever os testes (falham)**

`apps/tuios-apps/tests/test_package.py`:
```python
import hashlib
import io
import tarfile
from pathlib import Path

import pytest

from tuiosapps.manifest import ErroManifesto
from tuiosapps.package import ErroPacote, adicionar, empacotar, verificar_checksum


def test_empacotar_gera_targz_com_prefixo_e_sums(app_valido: Path, tmp_path: Path):
    saida = tmp_path / "app.tar.gz"
    resultado = empacotar(app_valido, saida)
    assert resultado == saida and saida.is_file()
    with tarfile.open(saida, "r:gz") as tar:
        nomes = tar.getnames()
    assert "ola-tuios/tuios-app.toml" in nomes
    assert "ola-tuios/ola.prw" in nomes
    assert "ola-tuios/SHA256SUMS" in nomes


def test_empacotar_exclui_lixo(app_valido: Path, tmp_path: Path):
    (app_valido / ".git").mkdir()
    (app_valido / ".git" / "config").write_text("x", encoding="utf-8")
    (app_valido / "__pycache__").mkdir()
    (app_valido / "__pycache__" / "a.pyc").write_bytes(b"x")
    (app_valido / "velho.pyc").write_bytes(b"x")
    saida = empacotar(app_valido, tmp_path / "app.tar.gz")
    with tarfile.open(saida, "r:gz") as tar:
        nomes = tar.getnames()
    assert not any(".git" in n or "__pycache__" in n or n.endswith(".pyc") for n in nomes)


def test_empacotar_manifesto_invalido_falha(app_valido: Path, tmp_path: Path):
    (app_valido / "tuios-app.toml").write_text("nome = [x", encoding="utf-8")
    with pytest.raises(ErroManifesto):
        empacotar(app_valido, tmp_path / "app.tar.gz")


def test_roundtrip_empacotar_adicionar(app_valido: Path, tmp_path: Path):
    destino = tmp_path / "usuario"
    destino.mkdir()
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    alvo = adicionar(tarball, destino)
    assert alvo == destino / "ola-tuios"
    assert (alvo / "tuios-app.toml").is_file()
    assert (alvo / "ola.prw").is_file()
    assert (alvo / "SHA256SUMS").is_file()


def test_adicionar_nao_gzip(tmp_path: Path):
    lixo = tmp_path / "lixo.tar.gz"
    lixo.write_bytes(b"nao sou gzip")
    with pytest.raises(ErroPacote, match="pacote invalido"):
        adicionar(lixo, tmp_path / "destino")


def test_adicionar_prefixos_multiplos_rejeitado(tmp_path: Path):
    # pacote com DOIS diretórios raiz → rejeita
    a = tmp_path / "origem"
    (a / "um").mkdir(parents=True)
    (a / "dois").mkdir(parents=True)
    (a / "um" / "t.txt").write_text("x", encoding="utf-8")
    (a / "dois" / "t.txt").write_text("x", encoding="utf-8")
    tarball = tmp_path / "dup.tar.gz"
    with tarfile.open(tarball, "w:gz") as tar:
        tar.add(a / "um", arcname="um")
        tar.add(a / "dois", arcname="dois")
    with pytest.raises(ErroPacote, match="unico diretorio"):
        adicionar(tarball, tmp_path / "destino")


def _tar_malicioso(tmp_path: Path, nome_entrada: str) -> Path:
    tarball = tmp_path / "malicioso.tar.gz"
    dados = b"pwn"
    with tarfile.open(tarball, "w:gz") as tar:
        ti = tarfile.TarInfo(name=nome_entrada)
        ti.size = len(dados)
        tar.addfile(ti, io.BytesIO(dados))
    return tarball


def test_adicionar_rejeita_path_traversal(tmp_path: Path):
    tarball = _tar_malicioso(tmp_path, "app/../escape.txt")
    with pytest.raises(ErroPacote, match="insegura"):
        adicionar(tarball, tmp_path / "destino")


def test_adicionar_rejeita_absoluto(tmp_path: Path):
    tarball = _tar_malicioso(tmp_path, "/etc/passwd")
    with pytest.raises(ErroPacote, match="insegura"):
        adicionar(tarball, tmp_path / "destino")


def test_adicionar_checksum_divergente(app_valido: Path, tmp_path: Path):
    # monta pacote e depois troca o conteúdo do entry sem atualizar SHA256SUMS
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    refeito = tmp_path / "refeito.tar.gz"
    with tarfile.open(tarball, "r:gz") as origem, tarfile.open(refeito, "w:gz") as novo:
        for m in origem.getmembers():
            dados = origem.extractfile(m).read() if m.isreg() else None
            if m.name == "ola-tuios/ola.prw":
                dados = b"Return .F.  // adulterado"
                m.size = len(dados)
            novo.addfile(m, io.BytesIO(dados) if dados is not None else None)
    with pytest.raises(ErroPacote, match="checksum"):
        adicionar(refeito, tmp_path / "destino")


def test_adicionar_sem_sums_rejeitado(app_valido: Path, tmp_path: Path):
    tarball = tmp_path / "sem-sums.tar.gz"
    with tarfile.open(tarball, "w:gz") as tar:
        tar.add(app_valido, arcname="ola-tuios")
        # remove SHA256SUMS caso tenha existido (não existe — só fontes)
    with pytest.raises(ErroPacote, match="SHA256SUMS"):
        adicionar(tarball, tmp_path / "destino")


def test_adicionar_app_ja_instalado(app_valido: Path, tmp_path: Path):
    destino = tmp_path / "usuario"
    destino.mkdir()
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    adicionar(tarball, destino)
    with pytest.raises(ErroPacote, match="ja instalado"):
        adicionar(tarball, destino)


def test_adicionar_manifesto_invalido_no_pacote(app_valido: Path, tmp_path: Path):
    # pacote válido é gerado; o manifesto é adulterado DENTRO do tarball
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    refeito = tmp_path / "refeito.tar.gz"
    invalido = 'nome = "ola-tuios"\nversao = "x"\ndescricao = "d"\nentry = "ola.prw"\n'
    with tarfile.open(tarball, "r:gz") as origem, tarfile.open(refeito, "w:gz") as novo:
        for m in origem.getmembers():
            dados = origem.extractfile(m).read() if m.isreg() else None
            if m.name == "ola-tuios/tuios-app.toml":
                dados = invalido.encode("utf-8")
                m.size = len(dados)
            novo.addfile(m, io.BytesIO(dados) if dados is not None else None)
    # manifesto é validado ANTES do checksum -> ErroManifesto (exit 2 no CLI)
    with pytest.raises(ErroManifesto):
        adicionar(refeito, tmp_path / "destino")


def test_verificar_checksum_ok(app_valido: Path, tmp_path: Path):
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    destino = tmp_path / "usuario"
    destino.mkdir()
    alvo = adicionar(tarball, destino)
    verificar_checksum(alvo)  # não levanta


def test_verificar_checksum_faltando_arquivo(app_valido: Path, tmp_path: Path):
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    destino = tmp_path / "usuario"
    destino.mkdir()
    alvo = adicionar(tarball, destino)
    (alvo / "ola.prw").unlink()
    with pytest.raises(ErroPacote, match="divergente"):
        verificar_checksum(alvo)


def test_remover_rmtree(app_valido: Path, tmp_path: Path):
    destino = tmp_path / "usuario"
    destino.mkdir()
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    alvo = adicionar(tarball, destino)
    import shutil

    shutil.rmtree(alvo)
    assert not alvo.exists()
```

Obs.: a remoção em si é `shutil.rmtree` no `cli` (Task 6) — aqui cobre-se o ciclo do `package`; o teste acima documenta o contrato do alvo.

- [X] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_package.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tuiosapps.package'`

- [X] **Step 3: Implementar `package.py`**

`apps/tuios-apps/tuiosapps/package.py`:
```python
"""Ciclo de pacote: empacotar / adicionar / remover (tar.gz + SHA256SUMS, R9–R11)."""
from __future__ import annotations

import hashlib
import io
import re
import shutil
import tarfile
import tempfile
from pathlib import Path, PurePosixPath

from .manifest import ErroManifesto, carregar_manifesto

NOME_SUMS = "SHA256SUMS"
EXCLUIR_DIRS = {".git", "__pycache__"}
EXCLUIR_SUFIXOS = (".pyc",)
LINHA_SUMS_RE = re.compile(r"^([0-9a-f]{64})  (.+)$")


class ErroPacote(Exception):
    """Pacote inválido/instalação recusada — o CLI converte em exit 2."""


def _incluir(rel: str) -> bool:
    partes = PurePosixPath(rel).parts
    if any(p in EXCLUIR_DIRS for p in partes):
        return False
    if rel.endswith(EXCLUIR_SUFIXOS):
        return False
    if partes and partes[-1] == NOME_SUMS:
        return False
    return True


def empacotar(dir_app: Path, saida: Path | None = None) -> Path:
    """Valida o manifesto e gera <saida>.tar.gz prefixado com o nome do app."""
    m = carregar_manifesto(dir_app)
    destino = saida if saida is not None else Path(f"{m.nome}-{m.versao}.tar.gz")

    arquivos = [
        p
        for p in sorted(dir_app.rglob("*"))
        if p.is_file() and _incluir(p.relative_to(dir_app).as_posix())
    ]
    linhas = [
        f"{hashlib.sha256(p.read_bytes()).hexdigest()}  "
        f"{p.relative_to(dir_app).as_posix()}"
        for p in arquivos
    ]
    sums = ("\n".join(linhas) + "\n").encode("utf-8")

    with tarfile.open(destino, "w:gz") as tar:
        for p in arquivos:
            tar.add(p, arcname=f"{m.nome}/{p.relative_to(dir_app).as_posix()}")
        ti = tarfile.TarInfo(name=f"{m.nome}/{NOME_SUMS}")
        ti.size = len(sums)
        ti.mtime = int(dir_app.stat().st_mtime)
        tar.addfile(ti, io.BytesIO(sums))
    return destino


def verificar_checksum(dir_app: Path) -> None:
    """Confere SHA256SUMS contra o conteúdo real (listagem exata, dois sentidos)."""
    arquivo = dir_app / NOME_SUMS
    if not arquivo.is_file():
        raise ErroPacote(f"{NOME_SUMS} ausente no pacote")
    esperados: dict[str, str] = {}
    for linha in arquivo.read_text(encoding="utf-8").splitlines():
        if not linha:
            continue
        m = LINHA_SUMS_RE.match(linha)
        if not m:
            raise ErroPacote(f"{NOME_SUMS} malformado: {linha!r}")
        esperados[m.group(2)] = m.group(1)

    atuais: dict[str, str] = {}
    for p in sorted(dir_app.rglob("*")):
        if not p.is_file():
            continue
        rel = p.relative_to(dir_app).as_posix()
        if rel == NOME_SUMS:
            continue
        atuais[rel] = hashlib.sha256(p.read_bytes()).hexdigest()

    if set(esperados) != set(atuais):
        faltando = sorted(set(esperados) - set(atuais))
        extra = sorted(set(atuais) - set(esperados))
        raise ErroPacote(
            f"{NOME_SUMS} divergente (faltando: {faltando}, extra: {extra})"
        )
    for rel, digest in esperados.items():
        if atuais[rel] != digest:
            raise ErroPacote(f"checksum divergente: {rel}")


def adicionar(tarball: Path, destino: Path) -> Path:
    """Valida o pacote em staging e só então instala em <destino>/<nome>."""
    if not tarball.is_file():
        raise ErroPacote(f"pacote nao encontrado: {tarball}")
    try:
        tar = tarfile.open(tarball, "r:gz")
    except (tarfile.ReadError, OSError) as exc:
        raise ErroPacote(f"pacote invalido (gzip/tar): {exc}") from exc

    with tar:
        membros = tar.getmembers()
        if not membros:
            raise ErroPacote("pacote vazio")
        prefixos = {
            PurePosixPath(m.name).parts[0]
            for m in membros
            if PurePosixPath(m.name).parts
        }
        if len(prefixos) != 1:
            raise ErroPacote(
                f"pacote deve ter um unico diretorio raiz; encontrado: {sorted(prefixos)}"
            )
        raiz = prefixos.pop()
        if raiz in (".", ".."):
            raise ErroPacote(f"raiz insegura no pacote: {raiz!r}")
        for m in membros:
            pp = PurePosixPath(m.name)
            if pp.is_absolute() or ".." in pp.parts:
                raise ErroPacote(f"entrada insegura: {m.name}")
            if m.issym() or m.islnk():
                alvo_link = PurePosixPath(m.linkname)
                if alvo_link.is_absolute() or ".." in alvo_link.parts:
                    raise ErroPacote(f"link inseguro: {m.name}")

        with tempfile.TemporaryDirectory(prefix="tuios-apps-") as tmp:
            try:
                tar.extractall(tmp, filter="data")
            except (tarfile.FilterError, ValueError) as exc:
                raise ErroPacote(f"extracao recusada: {exc}") from exc
            candidato = Path(tmp) / raiz
            try:
                m = carregar_manifesto(candidato)
            except ErroManifesto:
                raise
            if m.nome != raiz:
                raise ErroPacote(
                    f"nome do manifesto ({m.nome}) difere da raiz do pacote ({raiz})"
                )
            verificar_checksum(candidato)

            destino.mkdir(parents=True, exist_ok=True)
            alvo_final = destino / raiz
            if alvo_final.exists():
                raise ErroPacote(f"app ja instalado: {m.nome}")
            shutil.move(str(candidato), str(alvo_final))
    return alvo_final
```

- [X] **Step 4: Rodar e verificar a passagem**

Run: `cd apps/tuios-apps && python3 -m pytest tests -q`
Expected: todos passam (arquivo `tests/test_package.py` inteiro)

- [X] **Step 5: Commit**

```bash
git add apps/tuios-apps
git commit -m "[FEAT] — ciclo de pacote tar.gz com SHA256SUMS e staging seguro (R9-R11)"
```

---

### Task 6: `cli.py` — argparse, exit codes, --json (R6, R12, R13 + orquestração R7–R11)

**Files:**
- Create: `apps/tuios-apps/tests/test_cli.py`
- Create: `apps/tuios-apps/tuiosapps/cli.py`

**Interfaces:**
- Consumes: todos os módulos anteriores; entrypoint da Task 1 chama `main`
- Produces: `main(argv: list[str] | None = None) -> int` — contrato de exit codes `0/1/2/3/4`

- [X] **Step 1: Escrever os testes (falham)**

`apps/tuios-apps/tests/test_cli.py`:
```python
import json
from pathlib import Path

import pytest

from tuiosapps import cli
from tuiosapps.cli import main


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


def test_listar_vazio_texto_e_json(roots_tmp, capsys):
    assert main(["listar"]) == 0
    assert capsys.readouterr().out == ""
    assert main(["listar", "--json"]) == 0
    assert json.loads(capsys.readouterr().out) == []


def test_listar_json_com_app(roots_tmp, app_valido: Path, capsys):
    _, usuario = roots_tmp
    _instalar_app(usuario, app_valido)
    assert main(["listar", "--json"]) == 0
    dados = json.loads(capsys.readouterr().out)
    assert dados[0]["nome"] == "ola-tuios"
    assert dados[0]["origem"] == "usuario"
    assert dados[0]["categoria"] == "exemplo"
    assert dados[0]["path"].endswith("ola-tuios")


def test_listar_texto_cabecalho(roots_tmp, app_valido: Path, capsys):
    _, usuario = roots_tmp
    _instalar_app(usuario, app_valido)
    assert main(["listar"]) == 0
    saida = capsys.readouterr().out
    assert "NOME" in saida and "ola-tuios" in saida and "usuario" in saida


def test_info_ok(roots_tmp, app_valido: Path, capsys):
    _, usuario = roots_tmp
    _instalar_app(usuario, app_valido)
    assert main(["info", "ola-tuios"]) == 0
    saida = capsys.readouterr().out
    assert "nome: ola-tuios" in saida
    assert "origem: usuario" in saida
    assert "entry: ola.prw" in saida


def test_info_nao_encontrado_exit3(roots_tmp, capsys):
    assert main(["info", "fantasma"]) == 3
    assert "nao encontrado" in capsys.readouterr().err


def test_validar_ok(roots_tmp, app_valido: Path, capsys):
    assert main(["validar", str(app_valido)]) == 0
    assert "ola-tuios" in capsys.readouterr().out


def test_validar_invalido_exit2(roots_tmp, tmp_path: Path, capsys):
    vazio = tmp_path / "vazio"
    vazio.mkdir()
    assert main(["validar", str(vazio)]) == 2
    assert "erro:" in capsys.readouterr().err


def test_empacotar_fluxo_completo(roots_tmp, app_valido: Path, tmp_path: Path, capsys):
    sistema, usuario = roots_tmp
    tarball = tmp_path / "saida.tar.gz"

    assert main(["empacotar", str(app_valido), "-o", str(tarball)]) == 0
    capsys.readouterr()
    assert tarball.is_file()

    assert main(["adicionar", str(tarball)]) == 0
    capsys.readouterr()
    assert (usuario / "ola-tuios" / "tuios-app.toml").is_file()

    assert main(["listar", "--json"]) == 0
    dados = json.loads(capsys.readouterr().out)
    assert dados[0]["nome"] == "ola-tuios"

    assert main(["remover", "ola-tuios"]) == 0
    capsys.readouterr()
    assert not (usuario / "ola-tuios").exists()


def test_adicionar_duplicado_exit2(roots_tmp, app_valido: Path, tmp_path: Path, capsys):
    from tuiosapps.package import empacotar

    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    assert main(["adicionar", str(tarball)]) == 0
    capsys.readouterr()
    assert main(["adicionar", str(tarball)]) == 2
    assert "ja instalado" in capsys.readouterr().err


def test_adicionar_sistema_sem_root_exit1(roots_tmp, app_valido: Path, tmp_path: Path, monkeypatch, capsys):
    from tuiosapps.package import empacotar

    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    monkeypatch.setattr(cli.os, "geteuid", lambda: 1000)
    assert main(["adicionar", str(tarball), "--sistema"]) == 1
    assert "root" in capsys.readouterr().err


def test_remover_nao_encontrado_exit3(roots_tmp, capsys):
    assert main(["remover", "fantasma"]) == 3
    assert "nao encontrado" in capsys.readouterr().err


def test_rodar_advplc_ausente_exit4(roots_tmp, app_valido: Path, tmp_path: Path, monkeypatch, capsys):
    _, usuario = roots_tmp
    _instalar_app(usuario, app_valido)
    vazio = tmp_path / "path-vazio"
    vazio.mkdir()
    monkeypatch.setenv("PATH", str(vazio))
    assert main(["rodar", "ola-tuios"]) == 4
    assert "advplc nao encontrado" in capsys.readouterr().err


def test_rodar_propaga_exit_code(roots_tmp, app_valido: Path, tmp_path: Path, monkeypatch, capsys):
    import stat

    _, usuario = roots_tmp
    _instalar_app(usuario, app_valido)
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    fake = bin_dir / "advplc"
    fake.write_text("#!/bin/sh\nexit 5\n", encoding="utf-8")
    fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
    monkeypatch.setenv("PATH", str(bin_dir))
    assert main(["rodar", "ola-tuios"]) == 5


def test_sem_comando_exit2(capsys):
    with pytest.raises(SystemExit) as exc:
        main([])
    assert exc.value.code == 2
```

- [X] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_cli.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'tuiosapps.cli'`

- [X] **Step 3: Implementar `cli.py`**

`apps/tuios-apps/tuiosapps/cli.py`:
```python
"""CLI tuios-apps — argparse + exit codes 0/1/2/3/4 + saída --json (R6, R12, R13)."""
from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
from pathlib import Path

from . import __version__
from .discovery import App, buscar, descobrir
from .manifest import ErroManifesto, carregar_manifesto
from .package import ErroPacote, adicionar, empacotar
from .runner import AdvplcAusente, rodar

EXIT_ERRO = 1
EXIT_INVALIDO = 2  # manifesto/pacote inválido
EXIT_NAO_ENCONTRADO = 3
EXIT_DEPENDENCIA = 4


class AppNaoEncontrado(Exception):
    """App inexistente — exit 3."""


def _erro(msg: str) -> None:
    print(f"tuios-apps: erro: {msg}", file=sys.stderr)


def _dirs_destino(sistema: bool) -> Path:
    from .discovery import roots

    alvo = "sistema" if sistema else "usuario"
    for origem, caminho in roots():
        if origem == alvo:
            return caminho
    raise AssertionError("roots() sem a origem pedida")


def _cmd_listar(args: argparse.Namespace) -> int:
    apps = descobrir()
    if args.json:
        print(
            json.dumps(
                [
                    {
                        "nome": a.nome,
                        "versao": a.versao,
                        "descricao": a.descricao,
                        "categoria": a.categoria,
                        "origem": a.origem,
                        "path": str(a.path),
                    }
                    for a in apps
                ],
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0
    if apps:
        print(f"{'NOME':<16} {'VERSAO':<8} {'ORIGEM':<8} DESCRICAO")
        for a in apps:
            print(f"{a.nome:<16} {a.versao:<8} {a.origem:<8} {a.descricao}")
    return 0


def _cmd_info(args: argparse.Namespace) -> int:
    app = buscar(args.nome)
    if app is None:
        raise AppNaoEncontrado(f"app nao encontrado: {args.nome}")
    print(f"nome: {app.nome}")
    print(f"versao: {app.versao}")
    print(f"descricao: {app.descricao}")
    print(f"categoria: {app.categoria or '-'}")
    print(f"autor: {app.autor or '-'}")
    print(f"origem: {app.origem}")
    print(f"entry: {app.entry}")
    print(f"path: {app.path}")
    return 0


def _cmd_rodar(args: argparse.Namespace) -> int:
    app = buscar(args.nome)
    if app is None:
        raise AppNaoEncontrado(f"app nao encontrado: {args.nome}")
    restante = list(args.resto)
    if restante and restante[0] == "--":
        restante = restante[1:]
    return rodar(app, restante)


def _cmd_validar(args: argparse.Namespace) -> int:
    m = carregar_manifesto(Path(args.dir))
    print(f"OK: {m.nome} {m.versao} (entry: {m.entry})")
    return 0


def _cmd_empacotar(args: argparse.Namespace) -> int:
    saida = empacotar(Path(args.dir), Path(args.saida) if args.saida else None)
    print(saida)
    return 0


def _cmd_adicionar(args: argparse.Namespace) -> int:
    if args.sistema and os.geteuid() != 0:
        raise PermissionError("adicionar --sistema exige root (use sudo)")
    destino = _dirs_destino(args.sistema)
    alvo = adicionar(Path(args.tarball), destino)
    print(alvo)
    return 0


def _cmd_remover(args: argparse.Namespace) -> int:
    if args.sistema and os.geteuid() != 0:
        raise PermissionError("remover --sistema exige root (use sudo)")
    origem_desejada = "sistema" if args.sistema else "usuario"
    alvo = next(
        (a for a in descobrir() if a.nome == args.nome and a.origem == origem_desejada),
        None,
    )
    if alvo is None:
        raise AppNaoEncontrado(
            f"app nao encontrado: {args.nome} (origem {origem_desejada})"
        )
    shutil.rmtree(alvo.path)
    print(alvo.path)
    return 0


def _montar_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="tuios-apps",
        description="Apps AdvPL de primeira classe no tuiOS (Nível C, fase 1)",
    )
    parser.add_argument("--version", action="version", version=f"tuios-apps {__version__}")
    sub = parser.add_subparsers(dest="comando", required=True)

    p = sub.add_parser("listar", help="lista apps instalados")
    p.add_argument("--json", action="store_true", help="saída JSON")
    p.set_defaults(func=_cmd_listar)

    p = sub.add_parser("info", help="mostra detalhes de um app")
    p.add_argument("nome")
    p.set_defaults(func=_cmd_info)

    p = sub.add_parser("rodar", help="executa um app via advplc")
    p.add_argument("nome")
    p.add_argument("resto", nargs=argparse.REMAINDER, help="argumentos após --")
    p.set_defaults(func=_cmd_rodar)

    p = sub.add_parser("validar", help="valida o manifesto de um diretório")
    p.add_argument("dir", nargs="?", default=".")
    p.set_defaults(func=_cmd_validar)

    p = sub.add_parser("empacotar", help="gera app.tar.gz a partir do diretório")
    p.add_argument("dir")
    p.add_argument("-o", "--saida", default=None, help="arquivo de saída")
    p.set_defaults(func=_cmd_empacotar)

    p = sub.add_parser("adicionar", help="instala um pacote .tar.gz")
    p.add_argument("tarball")
    p.add_argument("--sistema", action="store_true", help="instala em /opt/tuios/apps (root)")
    p.set_defaults(func=_cmd_adicionar)

    p = sub.add_parser("remover", help="remove um app instalado")
    p.add_argument("nome")
    p.add_argument("--sistema", action="store_true", help="remove de /opt/tuios/apps (root)")
    p.set_defaults(func=_cmd_remover)

    return parser


def main(argv: list[str] | None = None) -> int:
    """Entry point: devolve o exit code (nunca lança para erro de uso)."""
    args = _montar_parser().parse_args(argv)
    try:
        return args.func(args)
    except (ErroManifesto, ErroPacote) as exc:
        _erro(str(exc))
        return EXIT_INVALIDO
    except AppNaoEncontrado as exc:
        _erro(str(exc))
        return EXIT_NAO_ENCONTRADO
    except AdvplcAusente as exc:
        _erro(str(exc))
        return EXIT_DEPENDENCIA
    except PermissionError as exc:
        _erro(str(exc))
        return EXIT_ERRO
    except BrokenPipeError:
        return 0
    except OSError as exc:
        _erro(f"falha de sistema: {exc}")
        return EXIT_ERRO
```

- [X] **Step 4: Rodar e verificar a passação**

Run: `cd apps/tuios-apps && python3 -m pytest tests -q`
Expected: todos passam

- [X] **Step 5: Commit**

```bash
git add apps/tuios-apps
git commit -m "[FEAT] — CLI tuios-apps: listar/info/rodar/validar/empacotar/adicionar/remover (R6, R12, R13)"
```

---

### Task 7: Assert e2e + integração no Justfile/CI (F7)

**Files:**
- Create: `scripts-assert/apps.assert.sh`
- Modify: `Justfile` (receitas `test-python`, `test-apps`; adicionar as duas linhas no `test-all` — hoje o `test-all` começa na linha 73 e lista os asserts entre as linhas 75-83)
- Modify: `.github/workflows/build.yml` (step de pytest antes de `Executar testes` / `run: just test-all`, hoje na linha ~48)

**Interfaces:**
- Consumes: entrypoint `apps/tuios-apps/tuios-apps` executável; contrato `APPS-ASSERT-OK`
- Produces: `just test-all` roda pytest + apps.assert; CI instala pytest

- [X] **Step 1: Escrever o assert (falha — arquivo não existe)**

`scripts-assert/apps.assert.sh`:
```bash
#!/usr/bin/env bash
# Ciclo completo do tuios-apps em tmpdir (sem QEMU): R1-R13 em env isolado.
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$RAIZ/apps/tuios-apps/tuios-apps"
command -v python3 >/dev/null || { echo "FALHA: python3 ausente"; exit 1; }
[ -f "$BIN" ] || { echo "FALHA: entrypoint ausente: $BIN"; exit 1; }

W="$(mktemp -d /tmp/tuios-apps-test.XXXXXX)"
trap 'rm -rf "$W"' EXIT

export TUIOS_APPS_SYSTEM="$W/sistema"
export TUIOS_APPS_USER="$W/usuario"
mkdir -p "$TUIOS_APPS_SYSTEM" "$TUIOS_APPS_USER"

falha() { echo "FALHA: $*"; exit 1; }

# 1. app de exemplo
APP="$W/origem/ola-tuios"
mkdir -p "$APP"
cat > "$APP/tuios-app.toml" << 'EOF'
nome = "ola-tuios"
versao = "1.0.0"
descricao = "Primeiro app tuiOS"
entry = "ola.prw"
categoria = "exemplo"
EOF
cat > "$APP/ola.prw" << 'EOF'
User Function Ola()
    ConOut("OLA-TUIOS-OK")
Return .T.
EOF

# 2. validar (R1/R13)
"$BIN" validar "$APP" >/dev/null || falha "validar"

# 3. empacotar (R9)
( cd "$W/origem" && "$BIN" empacotar ola-tuios -o "$W/ola-tuios-1.0.0.tar.gz" >/dev/null ) \
  || falha "empacotar"
[ -f "$W/ola-tuios-1.0.0.tar.gz" ] || falha "tarball nao gerado"

# 4. adicionar (R10) — destino padrao = usuario
"$BIN" adicionar "$W/ola-tuios-1.0.0.tar.gz" >/dev/null || falha "adicionar"
[ -f "$TUIOS_APPS_USER/ola-tuios/tuios-app.toml" ] || falha "app nao instalado no dir usuario"

# 5. listar --json parseavel (R6)
JSON="$("$BIN" listar --json)" || falha "listar --json"
echo "$JSON" | python3 -c '
import json, sys
dados = json.load(sys.stdin)
assert any(a["nome"] == "ola-tuios" and a["origem"] == "usuario" for a in dados), dados
' || falha "JSON sem ola-tuios"

# 6. rodar: aceita 0 (executou) ou 4 (advplc ausente fora da ISO) (R7/R8)
set +e
"$BIN" rodar ola-tuios >/dev/null 2>&1
RC=$?
set -e
case "$RC" in
  0|4) ;;
  *) falha "rodar retornou $RC (esperado 0 ou 4)" ;;
esac

# 7. exit codes (R2/R3)
set +e
"$BIN" info fantasma >/dev/null 2>&1; RC1=$?
"$BIN" validar "$W/nao-existe" >/dev/null 2>&1; RC2=$?
set -e
[ "$RC1" -eq 3 ] || falha "info inexistente exit=$RC1 (esperado 3)"
[ "$RC2" -eq 2 ] || falha "validar invalido exit=$RC2 (esperado 2)"

# 8. remover (R11)
"$BIN" remover ola-tuios >/dev/null || falha "remover"
[ ! -e "$TUIOS_APPS_USER/ola-tuios" ] || falha "remover nao removeu o app"

# 9. sombra: app do usuario esconde o do sistema (R5)
mkdir -p "$TUIOS_APPS_SYSTEM/ola-tuios"
cp "$APP/tuios-app.toml" "$APP/ola.prw" "$TUIOS_APPS_SYSTEM/ola-tuios/"
mkdir -p "$TUIOS_APPS_USER/ola-tuios"
sed 's/1.0.0/9.9.9/' "$APP/tuios-app.toml" > "$TUIOS_APPS_USER/ola-tuios/tuios-app.toml"
cp "$APP/ola.prw" "$TUIOS_APPS_USER/ola-tuios/"
VERSAO="$("$BIN" listar --json | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d[0]["versao"], d[0]["origem"])')"
[ "$VERSAO" = "9.9.9 usuario" ] || falha "sombra usuario>sistema nao vale (obteve: $VERSAO)"

echo "APPS-ASSERT-OK"
```

- [X] **Step 2: Tornar executável e rodar (o CLI da Task 6 já existe — deve passar)**

Run:
```bash
chmod +x scripts-assert/apps.assert.sh apps/tuios-apps/tuios-apps
bash scripts-assert/apps.assert.sh
```
Expected: `APPS-ASSERT-OK`

Se falhar, a mensagem `FALHA: ...` aponta o passo exato do ciclo — corrigir o CLI/assert antes de prosseguir.

- [X] **Step 4: Integrar no Justfile**

No `Justfile`, após a receita `test-nodev` (linha ~64) e antes de `test-install`, adicionar:

```make
# Testes unitários do tuios-apps (Python)
test-python:
    cd apps/tuios-apps && python3 -m pytest tests -q

# Assert do ciclo completo do tuios-apps
test-apps:
    bash scripts-assert/apps.assert.sh
```

No recipe `test-all`, após a linha `@bash scripts-assert/nodev.assert.sh` adicionar:

```make
    @cd apps/tuios-apps && python3 -m pytest tests -q
    @bash scripts-assert/apps.assert.sh
```

- [X] **Step 5: pytest no CI**

No `.github/workflows/build.yml`, imediatamente **antes** do step `- name: Executar testes` (`run: just test-all`), adicionar:

```yaml
    - name: Instalar pytest (tuios-apps)
      run: python3 -m pip install --user pytest
```

- [X] **Step 6: Rodar a regressão completa**

Run: `just test-all`
Expected: `APPS-ASSERT-OK` aparece junto dos demais e o bloco final `TODOS OS TESTES PASSARAM! ✅`

- [X] **Step 7: Commit**

```bash
git add scripts-assert/apps.assert.sh Justfile .github/workflows/build.yml
git commit -m "[TEST] — apps.assert.sh + pytest no just test-all e no CI (F7)"
```

---

### Task 8: Empacotamento Nix — `tuios-apps` na ISO e no sistema instalado

**Files:**
- Create: `nixos/apps/tuios-apps.nix`
- Modify: `flake.nix` — no `let` (linha ~18, junto do `advplc`), em `packages` (linha ~34) e em `specialArgs` (linha ~45)
- Modify: `nixos/iso.nix` — args (linha 1), `environment.systemPackages` (linha ~24) e texto de `tuios-env.nix` (linha ~95)
- Modify: `nixos/instalado/configuration.nix` — `environment.systemPackages` (linha ~25)

**Interfaces:**
- Consumes: `apps/tuios-apps/` (pyproject com `[project.scripts]`); padrão existente do `advplc` (flake → specialArgs → iso.nix → tuios-env.nix → `env.*` no destino)
- Produces: pacote Nix `tuiosApps`; binário `tuios-apps` no PATH da ISO e do destino

- [X] **Step 1: Derivação**

`nixos/apps/tuios-apps.nix`:
```nix
# CLI tuios-apps — núcleo de apps AdvPL de primeira classe (Nível C, fase 1).
# Python puro (tomllib stdlib) → build pyproject sem rede (nixpkgs 24.05).
{ lib, python3Packages }:

python3Packages.buildPythonApplication {
  pname = "tuios-apps";
  version = "1.0.0";
  src = ../../apps/tuios-apps;
  format = "pyproject";
  nativeBuildInputs = [ python3Packages.setuptools ];
  nativeCheckInputs = [ python3Packages.pytest ];
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
```

- [X] **Step 2: Build do pacote (deve compilar + rodar os pytest dentro do sandbox)**

Run:
```bash
OUT=$(nix --store /tmp/nix-official build .#tuios-apps --print-out-paths --no-link)
"$OUT/bin/tuios-apps" --version
```
Expected: caminho `/tmp/nix-official/nix/store/...-tuios-apps-1.0.0` e depois `tuios-apps 1.0.0`
(obs: `--no-link` porque o symlink `result` lógico quebra com o `--store` custom desta máquina)

- [X] **Step 3: `flake.nix`**

No `let` (após linha 18):
```nix
      # Gerenciador de apps AdvPL (Nível C, fase 1 — ver apps/tuios-apps)
      tuiosApps = pkgs.callPackage ./nixos/apps/tuios-apps.nix { };
```
Em `packages.${system}` (após `inherit advplc;`):
```nix
        # Gerenciador de apps AdvPL
        inherit tuiosApps;
```
Em `specialArgs` (linha 45):
```nix
        specialArgs = { inherit tuios advplc tuiosApps; nixpkgsPath = nixpkgs.outPath; nixpkgsSrc = nixpkgs; };
```

- [X] **Step 4: `nixos/iso.nix`**

Args (linha 1):
```nix
{ modulesPath, pkgs, lib, tuios, advplc, tuiosApps, nixpkgsPath, nixpkgsSrc, ... }:
```
Após o `advplc` em `environment.systemPackages` (linha ~24):
```nix
    # Gerenciador de apps AdvPL (Nível C, fase 1)
    tuiosApps
```
Em `tuios-env.nix` (após linha 98):
```nix
      tuiosApps = builtins.storePath ${tuiosApps};
```

- [X] **Step 5: `nixos/instalado/configuration.nix`**

Em `environment.systemPackages` (após `env.advplc`):
```nix
    env.tuiosApps
```

- [X] **Step 6: Validação de eval (rápida) + build da ISO completa**

Run: `nix --store /tmp/nix-official build .#nixosConfigurations.iso.config.system.build.isoImage --dry-run`
Expected: avalia sem erro de sintaxe/arg (dry-run não baixa tudo)

Run: `just iso`
Expected: ISO nova gerada no store (contém `tuios-apps`)

- [X] **Step 7: Smoke manual na ISO (opcional, rápido)** — verificado por evidência equivalente: (a) closure do toplevel da ISO contém `/nix/store/c8hc5a9w83986vqf6vlqwxmdz8jyggc5-tuios-apps-1.0.0` (`nix path-info -r`); (b) o T2 da Task 9 executou `tuios-apps listar --json` → `[]` no sistema instalado a partir do store da ISO.

Run (QEMU mínimo com serial, esperar prompt):
```bash
QEMU com a ISO nova; no prompt: tuios-apps --version; tuios-apps listar --json
```
Expected: `tuios-apps 1.0.0` e `[]`

- [X] **Step 8: Commit**

```bash
git add nixos/apps/tuios-apps.nix flake.nix nixos/iso.nix nixos/instalado/configuration.nix
git commit -m "[FEAT] — tuios-apps empacotado no Nix: ISO e sistema instalado"
```

---

### Task 9: Smoke QEMU no T2 do install.assert (F7) + regressão final

**Files:**
- Modify: `scripts-assert/install.assert.sh:103-111` (comando do T2 e asserts)

**Interfaces:**
- Consumes: `tuios-apps` no PATH do sistema instalado (Task 8)
- Produce: T2 prova que o binário existe e responde JSON vazio

- [X] **Step 1: Estender o comando do T2**

Em `scripts-assert/install.assert.sh`, trocar:
```bash
enviar "systemctl is-active tuios-session; hostname; advplc --version 2>&1 | head -1; echo T2-FIM"
```
por:
```bash
enviar "systemctl is-active tuios-session; hostname; advplc --version 2>&1 | head -1; tuios-apps listar --json; echo T2-FIM"
```

- [X] **Step 2: Acrescentar o assert**

Após a linha do assert do advplc (`grep -qE "^advplc (dev|v[0-9]+\.)"`), adicionar:
```bash
# tuios-apps instalado e respondendo (JSON vazio — exemplos chegam na fase c-tui)
limpar_log | grep -q '^\[\]$' || { echo "FALHA: tuios-apps listar --json nao retornou []"; limpar_log | tail -30; exit 1; }
```

- [X] **Step 3: Regressão completa (obrigatória)**

Run: `just test-all`
Expected: `TODOS OS TESTES PASSARAM! ✅` (agora com pytest + APPS-ASSERT-OK)

Run: `just test-install`
Expected: `INSTAL-TEST-OK (T1 instalação + T2 boot)`

- [X] **Step 4: Commit**

```bash
git add scripts-assert/install.assert.sh
git commit -m "[TEST] — smoke do tuios-apps no T2 do install.assert (F7)"
```

---

## Self-Review (do próprio plano)

**1. Cobertura da spec (R1–R13):** R1,R2→Task 2 · R3,R4,R5→Task 3 (+9.9 do assert) · R6→Task 6 · R7,R8→Task 4 · R9→Task 5 · R10→Task 5 · R11→Task 5/6 · R12→Task 6 · R13→Task 6 · F7 testes→Tasks 1,7,9 · Nix→Task 8. **Sem lacunas.**

**2. Placeholders:** nenhum TBD/TODO; todo passo de código tem bloco completo; passos de run têm comando e saída esperada.

**3. Consistência de tipos:** `carregar_manifesto(dir_app: Path) -> Manifest` (Tasks 2→5→6) · `descobrir()/buscar()` (3→4→6) · `rodar(app, args) -> int` (4→6) · `empacotar/adicionar -> Path` (5→6) · `main(argv) -> int` (6→1) · mensagens casam com os `match=` dos testes (`manifesto nao encontrado`, `chave desconhecida`, `unica diretorio`→"unico diretorio", `insegura`, `checksum`, `ja instalado`, `app nao encontrado`, `advplc nao encontrado`) ✓.
