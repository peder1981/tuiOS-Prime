# tuios-apps Fase 3 (remoto) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Origens remotas e índice: `adicionar` com `http(s)/file://` e git,
`atualizar-indice`/`buscar`/`instalar` com `indice.toml` + sha256 — rede apenas
sob comando explícito, timeouts obrigatórios (spec R21–R26).

**Architecture:** dois módulos novos puros (`origem.py`: download/clone →
tarball; `indice.py`: schema/validação/sha256) + wiring na CLI existente.
Nenhuma mudança no manifesto, no menu nem no instalador — os exit codes
0/1/2/3/4 da Fase 1 são reaproveitados por mapeamento de exceções.

**Tech Stack:** stdlib (`urllib`, `subprocess`, `tomllib`, `hashlib`,
`dataclass`), `git` via `nativeCheckInputs` no Nix (testes reais no sandbox),
`http.server` em thread para o caminho http (skip se loopback indisponível).

## Global Constraints

- Mensagens e docs em **PT-BR**; identificadores técnicos em inglês/abreviações.
- Exit codes: `0` ok · `1` rede/índice-ausente · `2` índice/pacote inválido ·
  `3` não encontrado · `4` dependência (`advplc`/`dialog`/`git`).
- **Zero dependências PyPI**; `subprocess` sempre com lista (`shell=False`).
- Timeout de rede **30 s** (`TIMEOUT_REDE`) e de `git clone` **120 s**
  (`TIMEOUT_GIT`) — constantes, nunca inline.
- Manifesto `tuios-app.toml` **inalterado**; menu (Fase 2) **inalterado**.
- Commits: `[FEAT|FIX|REF|DOC|STYLE|TEST|CFG|PERF] — descrição curta` PT-BR,
  **sem** qualquer atribuição de assistente.
- Nix desta máquina: `nix --store /tmp/nix-official ... --print-out-paths
  --no-link`; caminho físico = prefixo `/tmp/nix-official`.
- Suíte esperada ao final: **95 passed** (64 atuais + 31 novos: 8 origem +
  11 índice + 12 remoto). Divergência → investigar antes de seguir.

---

### Task 1: `origem.py` — download + clone git (TDD)

**Files:**
- Create: `apps/tuios-apps/tests/test_origem.py`
- Create: `apps/tuios-apps/tuiosapps/origem.py`
- Modify: `nixos/apps/tuios-apps.nix` (git em `nativeCheckInputs`)

**Interfaces:**
- Consumes: `package.empacotar` (Fase 1, `.git` já excluído do pacote).
- Produces: `eh_remoto(str)->bool` · `baixar(url, destino)->Path` ·
  `materializar(origem, tmp)->Path` · exceções `RedeFalhou` (exit 1),
  `OrigemInvalida` (exit 1), `GitAusente` (exit 4).

- [ ] **Step 1: Escrever `tests/test_origem.py` (falha — módulo não existe)**

```python
import functools
import http.server
import shutil
import subprocess
import tarfile
import threading
from pathlib import Path

import pytest

from tuiosapps.origem import (
    GitAusente,
    OrigemInvalida,
    RedeFalhou,
    materializar,
)

TOML = (
    'nome = "ola-tuios"\n'
    'versao = "1.0.0"\n'
    'descricao = "Primeiro app tuiOS"\n'
    'entry = "ola.prw"\n'
)
PRW = 'User Function Ola()\nReturn .T.\n'
TEM_GIT = shutil.which("git") is not None


def test_materializar_file_url(tmp_path: Path):
    fonte = tmp_path / "fonte.bin"
    fonte.write_bytes(b"conteudo-remoto")
    saida = materializar(fonte.as_uri(), tmp_path / "work")
    assert saida == tmp_path / "work" / "pacote.tar.gz"
    assert saida.read_bytes() == b"conteudo-remoto"


def test_materializar_esquema_invalido(tmp_path: Path):
    with pytest.raises(OrigemInvalida):
        materializar("ftp://exemplo.invalido/x.tar.gz", tmp_path / "work")


def test_baixar_urlopen_falha(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    import urllib.error

    def boom(*args, **kwargs):
        raise urllib.error.URLError("sem rota")

    monkeypatch.setattr("urllib.request.urlopen", boom)
    with pytest.raises(RedeFalhou):
        materializar("https://exemplo.invalido/x.tar.gz", tmp_path / "work")


def test_baixar_download_vazio(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    class _Vazio:
        def __enter__(self):
            return self

        def __exit__(self, *args):
            return False

        def read(self, tamanho=None):
            return b""

    monkeypatch.setattr("urllib.request.urlopen", lambda *a, **k: _Vazio())
    with pytest.raises(RedeFalhou, match="vazio"):
        materializar("https://exemplo.invalido/x.tar.gz", tmp_path / "work")
    assert not (tmp_path / "work" / "pacote.tar.gz").exists()


def test_materializar_http_local(tmp_path: Path):
    fonte = tmp_path / "fonte.bin"
    fonte.write_bytes(b"http-conteudo")
    handler = functools.partial(
        http.server.SimpleHTTPRequestHandler, directory=str(tmp_path)
    )
    try:
        servidor = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    except OSError:
        pytest.skip("loopback indisponivel neste ambiente")
    thread = threading.Thread(target=servidor.serve_forever, daemon=True)
    thread.start()
    try:
        porta = servidor.server_address[1]
        saida = materializar(
            f"http://127.0.0.1:{porta}/fonte.bin", tmp_path / "work"
        )
        assert saida.read_bytes() == b"http-conteudo"
    finally:
        servidor.shutdown()
        servidor.server_close()


def _repo_git(base: Path) -> Path:
    repo = base / "meu-repo.git"
    repo.mkdir()
    ident = ["-c", "user.name=t", "-c", "user.email=t@t"]
    subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
    (repo / "tuios-app.toml").write_text(TOML, encoding="utf-8")
    (repo / "ola.prw").write_text(PRW, encoding="utf-8")
    subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
    subprocess.run(
        ["git", *ident, "commit", "-q", "-m", "inicio"], cwd=repo, check=True
    )
    return repo


@pytest.mark.skipif(not TEM_GIT, reason="git indisponivel")
def test_materializar_git_clone(tmp_path: Path):
    repo = _repo_git(tmp_path)
    saida = materializar(str(repo), tmp_path / "work")
    with tarfile.open(saida) as tar:
        nomes = tar.getnames()
    assert any(n.startswith("ola-tuios/") for n in nomes)
    assert not any(".git" in n.split("/") for n in nomes)


@pytest.mark.skipif(not TEM_GIT, reason="git indisponivel")
def test_materializar_git_clone_falha(tmp_path: Path):
    with pytest.raises(RedeFalhou, match="git clone falhou"):
        materializar(str(tmp_path / "nao-existe.git"), tmp_path / "work")


def test_materializar_git_ausente(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setattr("tuiosapps.origem.shutil.which", lambda *a: None)
    with pytest.raises(GitAusente):
        materializar("https://exemplo.invalido/app.git", tmp_path / "work")
```

- [ ] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_origem.py -q`
Expected: erro de coleção `ModuleNotFoundError: No module named 'tuiosapps.origem'`.

- [ ] **Step 3: Implementar `tuiosapps/origem.py`**

```python
"""Origens remotas: download http(s)/file e clone git (R21–R22, R26)."""
from __future__ import annotations

import os
import shutil
import subprocess
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import urlparse

from .package import empacotar

TIMEOUT_REDE = 30.0
TIMEOUT_GIT = 120.0


class RedeFalhou(Exception):
    """Download/clone falhou — o CLI converte em exit 1."""


class OrigemInvalida(Exception):
    """Origem com esquema não suportado — o CLI converte em exit 1."""


class GitAusente(Exception):
    """git fora do PATH — o CLI converte em exit 4."""


def eh_remoto(origem: str) -> bool:
    """URL http(s)/file ou origem git (.git) — decide download/clonagem."""
    if origem.endswith(".git"):
        return True
    return urlparse(origem).scheme in ("http", "https", "file")


def baixar(url: str, destino: Path) -> Path:
    """Baixa url para destino com timeout (lista de args, sem shell).

    Falha de rede/HTTP ou arquivo vazio => RedeFalhou (destino removido).
    """
    destino.parent.mkdir(parents=True, exist_ok=True)
    try:
        with urllib.request.urlopen(url, timeout=TIMEOUT_REDE) as resposta:
            with destino.open("wb") as arquivo:
                shutil.copyfileobj(resposta, arquivo)
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        destino.unlink(missing_ok=True)
        raise RedeFalhou(f"falha ao baixar {url}: {exc}") from exc
    if destino.stat().st_size == 0:
        destino.unlink(missing_ok=True)
        raise RedeFalhou(f"download vazio: {url}")
    return destino


def _clonar(url: str, destino: Path) -> None:
    if shutil.which("git") is None:
        raise GitAusente("git nao encontrado no PATH (instale git)")
    env = {**os.environ, "GIT_TERMINAL_PROMPT": "0"}
    try:
        r = subprocess.run(
            ["git", "clone", "--depth", "1", url, str(destino)],
            capture_output=True,
            text=True,
            timeout=TIMEOUT_GIT,
            env=env,
            shell=False,
        )
    except subprocess.TimeoutExpired as exc:
        raise RedeFalhou(
            f"git clone expirou apos {int(TIMEOUT_GIT)}s: {url}"
        ) from exc
    except OSError as exc:
        raise RedeFalhou(f"git clone falhou: {exc}") from exc
    if r.returncode != 0:
        linhas = (r.stderr or r.stdout).strip().splitlines()
        ultimo = linhas[-1] if linhas else f"rc={r.returncode}"
        raise RedeFalhou(f"git clone falhou: {ultimo[:300]}")


def materializar(origem: str, tmp: Path) -> Path:
    """Transforma origem em tarball dentro de tmp: download ou clone+empacotar."""
    tmp.mkdir(parents=True, exist_ok=True)
    if origem.endswith(".git"):
        repo = tmp / "repo"
        _clonar(origem, repo)
        return empacotar(repo, tmp / "pacote.tar.gz")
    if urlparse(origem).scheme in ("http", "https", "file"):
        return baixar(origem, tmp / "pacote.tar.gz")
    raise OrigemInvalida(f"origem nao suportada (http/https/file/.git): {origem}")
```

- [ ] **Step 4: Rodar e verificar a passagem**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_origem.py -q`
Expected: `8 passed` (ou `6 passed, 2 skipped` se `git` ausente no host).

- [ ] **Step 5: git no checkPhase do Nix**

Em `nixos/apps/tuios-apps.nix`:

1. Assinatura: `{ lib, python3Packages }:` → `{ lib, python3Packages, git }:`
2. `nativeCheckInputs = [ python3Packages.pytest ];` →
   `nativeCheckInputs = [ python3Packages.pytest git ];`

- [ ] **Step 6: Prova no sandbox Nix (checkPhase roda os testes)**

Run: `nix --store /tmp/nix-official build .#tuiosApps --no-link 2>&1 | tail -3`
Expected: termina sem erro (checkPhase com os testes de origem verdes —
inclui os 2 de git, pois `git` agora está no PATH do build).
Se falhar: `nix --store /tmp/nix-official log <drv>` mostra o teste.

- [ ] **Step 7: Commit**

```bash
git add apps/tuios-apps nixos/apps/tuios-apps.nix
git commit -m "[FEAT] — origem remota: download http(s)/file e clone git (R21, R22, R26)"
```

---

### Task 2: `indice.py` — índice local + sha256 (TDD)

**Files:**
- Create: `apps/tuios-apps/tests/test_indice.py`
- Create: `apps/tuios-apps/tuiosapps/indice.py`

**Interfaces:**
- Consumes: `origem.baixar` (Task 1), `manifest.SLUG_RE/VERSAO_RE` (Fase 1).
- Produces: `caminho_indice()->Path` · `atualizar(url)->Path` ·
  `carregar()->list[Entrada]` · `buscar(termo)` · `lookup(nome)` ·
  `baixar_e_conferir(entrada, tmp)->Path` · `Entrada` (frozen dataclass) ·
  exceções `ErroIndice` (exit 2), `IndiceNaoEncontrado` (exit 1).
  Store do índice: `${XDG_DATA_HOME:-~/.local/share}/tuios/indice.toml`.

- [ ] **Step 1: Escrever `tests/test_indice.py` (falha — módulo não existe)**

```python
import hashlib
from pathlib import Path

import pytest

from tuiosapps.indice import (
    ErroIndice,
    IndiceNaoEncontrado,
    atualizar,
    baixar_e_conferir,
    buscar,
    caminho_indice,
    carregar,
    lookup,
)

SHA_OK = "a" * 64
INDICE_OK = (
    "[[app]]\n"
    'nome = "ola-tuios"\n'
    'versao = "1.0.0"\n'
    'descricao = "Primeiro app tuiOS"\n'
    'url = "https://exemplo.invalido/ola-tuios-1.0.0.tar.gz"\n'
    f'sha256 = "{SHA_OK}"\n'
    "\n"
    "[[app]]\n"
    'nome = "outro-app"\n'
    'versao = "2.0.0"\n'
    'descricao = "Segundo app utilitario"\n'
    'url = "https://exemplo.invalido/outro-app-2.0.0.tar.gz"\n'
    f'sha256 = "{SHA_OK}"\n'
)


@pytest.fixture
def xdg(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    base = tmp_path / "xdg"
    monkeypatch.setenv("XDG_DATA_HOME", str(base))
    return base


def _publicar(tmp_path: Path, conteudo: str) -> str:
    fonte = tmp_path / "indice-remoto.toml"
    fonte.write_text(conteudo, encoding="utf-8")
    return fonte.as_uri()


def test_atualizar_indice_ok(xdg: Path, tmp_path: Path):
    atualizar(_publicar(tmp_path, INDICE_OK))
    assert caminho_indice().is_file()
    entradas = carregar()
    assert [e.nome for e in entradas] == ["ola-tuios", "outro-app"]
    assert entradas[0].sha256 == SHA_OK


def test_atualizar_sha_invalido_nao_grava(xdg: Path, tmp_path: Path):
    ruim = INDICE_OK.replace(SHA_OK, "b" * 63, 1)
    with pytest.raises(ErroIndice, match="sha256"):
        atualizar(_publicar(tmp_path, ruim))
    assert not caminho_indice().exists()


def test_atualizar_url_sem_esquema(xdg: Path, tmp_path: Path):
    ruim = INDICE_OK.replace("https://", "ftp://", 1)
    with pytest.raises(ErroIndice, match="esquema"):
        atualizar(_publicar(tmp_path, ruim))
    assert not caminho_indice().exists()


def test_atualizar_campos_faltando(xdg: Path, tmp_path: Path):
    ruim = INDICE_OK.replace('descricao = "Primeiro app tuiOS"\n', "", 1)
    with pytest.raises(ErroIndice, match="campos|sem"):
        atualizar(_publicar(tmp_path, ruim))


def test_atualizar_app_duplicado(xdg: Path, tmp_path: Path):
    ruim = INDICE_OK + '\n[[app]]\nnome = "ola-tuios"\nversao = "9.9.9"\ndescricao = "x"\nurl = "https://e.invalido/x.tar.gz"\n' + f'sha256 = "{SHA_OK}"\n'
    with pytest.raises(ErroIndice, match="duplicado"):
        atualizar(_publicar(tmp_path, ruim))


def test_carregar_sem_indice(xdg: Path):
    with pytest.raises(IndiceNaoEncontrado, match="atualizar-indice"):
        carregar()


def test_buscar_filtro(xdg: Path, tmp_path: Path):
    atualizar(_publicar(tmp_path, INDICE_OK))
    assert [e.nome for e in buscar("primeiro")] == ["ola-tuios"]
    assert [e.nome for e in buscar("UTILITARIO")] == ["outro-app"]


def test_buscar_sem_correspondencia(xdg: Path, tmp_path: Path):
    atualizar(_publicar(tmp_path, INDICE_OK))
    assert buscar("inexistente") == []


def test_lookup_nao_achado(xdg: Path, tmp_path: Path):
    atualizar(_publicar(tmp_path, INDICE_OK))
    assert lookup("fantasma") is None
    assert lookup("ola-tuios").versao == "1.0.0"


def test_baixar_e_conferir_ok(xdg: Path, tmp_path: Path):
    tarball = tmp_path / "app.tar.gz"
    tarball.write_bytes(b"pacote-legal")
    sha = hashlib.sha256(b"pacote-legal").hexdigest()
    conteudo = INDICE_OK.replace(
        "https://exemplo.invalido/ola-tuios-1.0.0.tar.gz", tarball.as_uri(), 1
    ).replace(SHA_OK, sha, 1)
    atualizar(_publicar(tmp_path, conteudo))
    entrada = lookup("ola-tuios")
    saida = baixar_e_conferir(entrada, tmp_path / "work")
    assert saida.read_bytes() == b"pacote-legal"


def test_baixar_e_conferir_sha_divergente(xdg: Path, tmp_path: Path):
    tarball = tmp_path / "app.tar.gz"
    tarball.write_bytes(b"pacote-adulterado")
    conteudo = INDICE_OK.replace(
        "https://exemplo.invalido/ola-tuios-1.0.0.tar.gz", tarball.as_uri(), 1
    )
    atualizar(_publicar(tmp_path, conteudo))  # sha no índice é SHA_OK (a*64)
    with pytest.raises(ErroIndice, match="sha256 divergente"):
        baixar_e_conferir(lookup("ola-tuios"), tmp_path / "work")
    assert not (tmp_path / "work" / "pacote.tar.gz").exists()
```

- [ ] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_indice.py -q`
Expected: coleção falha `ModuleNotFoundError: tuiosapps.indice`.

- [ ] **Step 3: Implementar `tuiosapps/indice.py`**

```python
"""Índice remoto de apps (R23–R25): atualizar / buscar / instalar com sha256."""
from __future__ import annotations

import hashlib
import os
import re
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlparse

from .manifest import SLUG_RE, VERSAO_RE
from .origem import baixar

SHA_RE = re.compile(r"^[0-9a-f]{64}$")
CHAVES = ("nome", "versao", "descricao", "url", "sha256")


class ErroIndice(Exception):
    """Índice inválido (schema/sha/url) — o CLI converte em exit 2."""


class IndiceNaoEncontrado(Exception):
    """Sem indice.toml local — o CLI converte em exit 1."""


@dataclass(frozen=True)
class Entrada:
    nome: str
    versao: str
    descricao: str
    url: str
    sha256: str


def caminho_indice() -> Path:
    base = os.environ.get("XDG_DATA_HOME") or str(Path.home() / ".local" / "share")
    return Path(base) / "tuios" / "indice.toml"


def _validar(bruta: object, onde: str) -> Entrada:
    if not isinstance(bruta, dict):
        raise ErroIndice(f"indice: entrada nao e uma tabela ({onde})")
    faltando = [k for k in CHAVES if k not in bruta]
    if faltando:
        raise ErroIndice(f"indice: entrada sem {', '.join(faltando)} ({onde})")
    nome = str(bruta["nome"])
    versao = str(bruta["versao"])
    if not SLUG_RE.match(nome):
        raise ErroIndice(f"indice: nome invalido ({nome!r})")
    if not VERSAO_RE.match(versao):
        raise ErroIndice(f"indice: versao invalida ({versao!r})")
    url = str(bruta["url"])
    if urlparse(url).scheme not in ("http", "https", "file"):
        raise ErroIndice(f"indice: url sem esquema http(s)/file ({url!r})")
    sha = str(bruta["sha256"]).lower()
    if not SHA_RE.match(sha):
        raise ErroIndice(f"indice: sha256 invalido ({nome})")
    return Entrada(nome, versao, str(bruta["descricao"]), url, sha)


def _validar_documento(dados: object) -> list[Entrada]:
    if not isinstance(dados, dict) or not isinstance(dados.get("app"), list):
        raise ErroIndice("indice: raiz precisa de [[app]]")
    entradas = [_validar(item, f"app[{i}]") for i, item in enumerate(dados["app"])]
    vistos = set()
    for e in entradas:
        if e.nome in vistos:
            raise ErroIndice(f"indice: app duplicado ({e.nome})")
        vistos.add(e.nome)
    return entradas


def atualizar(url: str) -> Path:
    """Baixa, valida e grava o indice local (nada é gravado se inválido)."""
    destino = caminho_indice()
    destino.parent.mkdir(parents=True, exist_ok=True)
    bruto = destino.parent / f".indice-{os.getpid()}.tmp"
    try:
        baixar(url, bruto)
        dados = tomllib.loads(bruto.read_text(encoding="utf-8"))
        _validar_documento(dados)
        bruto.replace(destino)
    finally:
        bruto.unlink(missing_ok=True)
    return destino


def carregar() -> list[Entrada]:
    p = caminho_indice()
    if not p.is_file():
        raise IndiceNaoEncontrado(
            f"indice nao encontrado: {p} "
            "(rode: tuios-apps atualizar-indice <url>)"
        )
    try:
        dados = tomllib.loads(p.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as exc:
        raise ErroIndice(f"indice corrompido: {exc}") from exc
    return _validar_documento(dados)


def buscar(termo: str) -> list[Entrada]:
    t = termo.lower()
    return [e for e in carregar() if t in e.nome.lower() or t in e.descricao.lower()]


def lookup(nome: str) -> Entrada | None:
    return next((e for e in carregar() if e.nome == nome), None)


def baixar_e_conferir(entrada: Entrada, tmp: Path) -> Path:
    """Baixa o tarball da entrada e confere o sha256 do índice (R25)."""
    tmp.mkdir(parents=True, exist_ok=True)
    tarball = tmp / "pacote.tar.gz"
    baixar(entrada.url, tarball)
    obtido = hashlib.sha256(tarball.read_bytes()).hexdigest()
    if obtido != entrada.sha256:
        tarball.unlink(missing_ok=True)
        raise ErroIndice(
            f"sha256 divergente para {entrada.nome}: "
            f"esperado {entrada.sha256}, obtido {obtido}"
        )
    return tarball
```

- [ ] **Step 4: Rodar e verificar a passagem**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_indice.py -q`
Expected: `11 passed`.

- [ ] **Step 5: Commit**

```bash
git add apps/tuios-apps
git commit -m "[FEAT] — indice local de apps com validacao e sha256 (R23, R24, R25)"
```

---

### Task 3: Wiring na CLI — `adicionar` remoto + `atualizar-indice`/`buscar`/`instalar` (TDD)

**Files:**
- Create: `apps/tuios-apps/tests/test_remoto.py`
- Modify: `apps/tuios-apps/tuiosapps/cli.py` (imports, 4 `_cmd_*`, subparsers, mapeamento de exceções)

**Interfaces:**
- Consumes: `origem.eh_remoto/materializar` + exceções; `indice.atualizar/
  buscar/lookup/baixar_e_conferir` + exceções; `package.adicionar` (inalterado).
- Produces: CLI `adicionar <url|origem-git>` (0/1/2/4),
  `atualizar-indice <url>` (0/1/2), `buscar <termo> [--json]` (0/1),
  `instalar <nome> [--sistema]` (0/1/2/3).

- [ ] **Step 1: Escrever `tests/test_remoto.py` (falha — subcomandos inexistentes)**

```python
import hashlib
import json
from pathlib import Path

import pytest

from tuiosapps.cli import main
from tuiosapps.package import empacotar

SHA_OK = "a" * 64
INDICE_OK = (
    "[[app]]\n"
    'nome = "ola-tuios"\n'
    'versao = "1.0.0"\n'
    'descricao = "Primeiro app tuiOS"\n'
    'url = "https://exemplo.invalido/ola-tuios-1.0.0.tar.gz"\n'
    f'sha256 = "{SHA_OK}"\n'
)


@pytest.fixture
def roots_tmp(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Path, Path]:
    sistema = tmp_path / "sistema"
    usuario = tmp_path / "usuario"
    sistema.mkdir()
    usuario.mkdir()
    monkeypatch.setenv("TUIOS_APPS_SYSTEM", str(sistema))
    monkeypatch.setenv("TUIOS_APPS_USER", str(usuario))
    return sistema, usuario


@pytest.fixture
def xdg(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    base = tmp_path / "xdg"
    monkeypatch.setenv("XDG_DATA_HOME", str(base))
    return base


def _indice_local(tmp_path: Path, tarball: Path, sha: str | None = None) -> str:
    digest = sha or hashlib.sha256(tarball.read_bytes()).hexdigest()
    conteudo = INDICE_OK.replace(
        "https://exemplo.invalido/ola-tuios-1.0.0.tar.gz", tarball.as_uri(), 1
    ).replace(SHA_OK, digest, 1)
    fonte = tmp_path / "indice-remoto.toml"
    fonte.write_text(conteudo, encoding="utf-8")
    return fonte.as_uri()


def test_adicionar_file_url(roots_tmp, app_valido: Path, tmp_path: Path):
    _, usuario = roots_tmp
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    assert main(["adicionar", tarball.as_uri()]) == 0
    assert (usuario / "ola-tuios" / "tuios-app.toml").is_file()


def test_adicionar_url_rede_falhou(roots_tmp, monkeypatch: pytest.MonkeyPatch):
    import urllib.error

    def boom(*args, **kwargs):
        raise urllib.error.URLError("sem rota")

    monkeypatch.setattr("urllib.request.urlopen", boom)
    assert main(["adicionar", "https://exemplo.invalido/a.tar.gz"]) == 1


def test_adicionar_git_ausente(monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setattr("tuiosapps.origem.shutil.which", lambda *a: None)
    assert main(["adicionar", "https://exemplo.invalido/app.git"]) == 4


def test_atualizar_indice_cli_ok(xdg, tmp_path: Path, capsys):
    fonte = tmp_path / "indice-remoto.toml"
    fonte.write_text(INDICE_OK, encoding="utf-8")
    assert main(["atualizar-indice", fonte.as_uri()]) == 0
    assert "indice.toml" in capsys.readouterr().out


def test_atualizar_indice_cli_invalido(xdg, tmp_path: Path):
    fonte = tmp_path / "indice-remoto.toml"
    fonte.write_text(INDICE_OK.replace(SHA_OK, "b" * 63, 1), encoding="utf-8")
    assert main(["atualizar-indice", fonte.as_uri()]) == 2
    from tuiosapps.indice import caminho_indice

    assert not caminho_indice().exists()


def test_buscar_cli_texto(xdg, tmp_path: Path, capsys):
    fonte = tmp_path / "indice-remoto.toml"
    fonte.write_text(INDICE_OK, encoding="utf-8")
    assert main(["atualizar-indice", fonte.as_uri()]) == 0
    capsys.readouterr()
    assert main(["buscar", "primeiro"]) == 0
    assert "ola-tuios" in capsys.readouterr().out


def test_buscar_cli_json(xdg, tmp_path: Path, capsys):
    fonte = tmp_path / "indice-remoto.toml"
    fonte.write_text(INDICE_OK, encoding="utf-8")
    assert main(["atualizar-indice", fonte.as_uri()]) == 0
    capsys.readouterr()
    assert main(["buscar", "ola", "--json"]) == 0
    dados = json.loads(capsys.readouterr().out)
    assert [d["nome"] for d in dados] == ["ola-tuios"]


def test_buscar_cli_sem_indice(xdg):
    assert main(["buscar", "qualquer"]) == 1


def test_instalar_ok(roots_tmp, xdg, app_valido: Path, tmp_path: Path):
    _, usuario = roots_tmp
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    assert main(["atualizar-indice", _indice_local(tmp_path, tarball)]) == 0
    assert main(["instalar", "ola-tuios"]) == 0
    assert (usuario / "ola-tuios" / "tuios-app.toml").is_file()


def test_instalar_nao_encontrado(roots_tmp, xdg, app_valido: Path, tmp_path: Path):
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    assert main(["atualizar-indice", _indice_local(tmp_path, tarball)]) == 0
    assert main(["instalar", "fantasma"]) == 3


def test_instalar_sem_indice(roots_tmp, xdg):
    assert main(["instalar", "ola-tuios"]) == 1


def test_instalar_sha_divergente(roots_tmp, xdg, app_valido: Path, tmp_path: Path):
    _, usuario = roots_tmp
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    assert main(["atualizar-indice", _indice_local(tmp_path, tarball, sha="b" * 64)]) == 0
    assert main(["instalar", "ola-tuios"]) == 2
    assert not (usuario / "ola-tuios").exists()
```

- [ ] **Step 2: Rodar e verificar a falha**

Run: `cd apps/tuios-apps && python3 -m pytest tests/test_remoto.py -q`
Expected: `12 failed` — argparse `SystemExit: 2` (subcomandos novos inexistentes).

- [ ] **Step 3: Ligar na CLI**

`apps/tuios-apps/tuiosapps/cli.py` — cinco blocos:

1. **Imports** (junto aos da Fase 2):

```python
from .indice import (
    ErroIndice,
    IndiceNaoEncontrado,
    atualizar as atualizar_indice,
    baixar_e_conferir,
    buscar as buscar_no_indice,
    lookup as lookup_indice,
)
from .origem import OrigemInvalida, RedeFalhou, GitAusente, eh_remoto, materializar
```

2. **`_cmd_adicionar` reescrito** (substituir o atual):

```python
def _cmd_adicionar(args: argparse.Namespace) -> int:
    if args.sistema and os.geteuid() != 0:
        raise PermissionError("adicionar --sistema exige root (use sudo)")
    destino = _dirs_destino(args.sistema)
    if eh_remoto(args.tarball):
        with tempfile.TemporaryDirectory(prefix="tuios-apps-") as td:
            tarball = materializar(args.tarball, Path(td))
            alvo = adicionar(tarball, destino)
    else:
        alvo = adicionar(Path(args.tarball), destino)
    print(alvo)
    return 0
```

3. **Três `_cmd_*` novos** (antes de `_montar_parser`):

```python
def _cmd_atualizar_indice(args: argparse.Namespace) -> int:
    print(atualizar_indice(args.url))
    return 0


def _cmd_buscar(args: argparse.Namespace) -> int:
    entradas = buscar_no_indice(args.termo)
    if args.json:
        print(
            json.dumps(
                [
                    {
                        "nome": e.nome,
                        "versao": e.versao,
                        "descricao": e.descricao,
                        "url": e.url,
                        "sha256": e.sha256,
                    }
                    for e in entradas
                ],
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0
    for e in entradas:
        print(f"{e.nome:<16} {e.versao:<8} {e.descricao}")
    return 0


def _cmd_instalar(args: argparse.Namespace) -> int:
    if args.sistema and os.geteuid() != 0:
        raise PermissionError("instalar --sistema exige root (use sudo)")
    destino = _dirs_destino(args.sistema)
    entrada = lookup_indice(args.nome)
    if entrada is None:
        raise AppNaoEncontrado(f"app nao encontrado no indice: {args.nome}")
    with tempfile.TemporaryDirectory(prefix="tuios-apps-") as td:
        tarball = baixar_e_conferir(entrada, Path(td))
        alvo = adicionar(tarball, destino)
    print(alvo)
    return 0
```

4. **Subparsers** (após o do `menu`), e help do `adicionar` atualizado para
   `"instala pacote .tar.gz, URL ou origem git"`:

```python
    p = sub.add_parser("atualizar-indice", help="baixa o indice remoto (indice.toml)")
    p.add_argument("url")
    p.set_defaults(func=_cmd_atualizar_indice)

    p = sub.add_parser("buscar", help="busca apps no indice local")
    p.add_argument("termo")
    p.add_argument("--json", action="store_true", help="saída JSON")
    p.set_defaults(func=_cmd_buscar)

    p = sub.add_parser("instalar", help="instala um app do indice")
    p.add_argument("nome")
    p.add_argument("--sistema", action="store_true", help="instala em /opt/tuios/apps (root)")
    p.set_defaults(func=_cmd_instalar)
```

5. **Mapeamento de exceções em `main()`** (após o `except DialogAusente`):

```python
    except (RedeFalhou, OrigemInvalida) as exc:
        _erro(str(exc))
        return EXIT_ERRO
    except IndiceNaoEncontrado as exc:
        _erro(str(exc))
        return EXIT_ERRO
    except ErroIndice as exc:
        _erro(str(exc))
        return EXIT_INVALIDO
    except GitAusente as exc:
        _erro(str(exc))
        return EXIT_DEPENDENCIA
```

Obs.: `import tempfile` entra no topo junto aos imports stdlib.

- [ ] **Step 4: Rodar a suíte completa**

Run: `cd apps/tuios-apps && python3 -m pytest tests -q`
Expected: **95 passed** (64 + 8 + 11 + 12). Divergência → investigar.

- [ ] **Step 5: Prova no sandbox Nix**

Run: `nix --store /tmp/nix-official build .#tuiosApps --no-link 2>&1 | tail -2`
Expected: sem erro (checkPhase com 95 testes verdes no sandbox).

- [ ] **Step 6: Commit**

```bash
git add apps/tuios-apps
git commit -m "[FEAT] — CLI com adicionar remoto, atualizar-indice, buscar e instalar (R21-R25)"
```

---

### Task 4: E2E sem rede no `apps.assert.sh` (file:// + índice)

**Files:**
- Modify: `scripts-assert/apps.assert.sh` (seção 11 antes de `APPS-ASSERT-OK`)

**Interfaces:**
- Consumes: seções 1-10 (estado final: app `ola-tuios` 9.9.9 no usuário);
  CLI da Task 3.
- Produz: evidência de `adicionar file://`, `atualizar-indice file://`,
  `buscar` e `instalar` em ambiente real (zero rede — R26).

- [ ] **Step 1: Acrescentar a seção 11**

Antes de `echo "APPS-ASSERT-OK"`:

```bash
# 11. origem remota sem rede: file:// + indice (R21, R23-R26)
"$BIN" remover ola-tuios >/dev/null || falha "limpeza p/ file://"
"$BIN" empacotar "$APP" -o "$W/pacote.tar.gz" >/dev/null || falha "empacotar p/ file://"
"$BIN" adicionar "file://$W/pacote.tar.gz" >/dev/null || falha "adicionar file://"
"$BIN" listar --json | grep -q '"nome": "ola-tuios"' || falha "file:// nao instalou"
"$BIN" remover ola-tuios >/dev/null || falha "remover apos file://"
SHA="$(sha256sum "$W/pacote.tar.gz" | cut -d' ' -f1)"
cat > "$W/indice-remoto.toml" << INDEOF
[[app]]
nome = "ola-tuios"
versao = "1.0.0"
descricao = "Primeiro app tuiOS"
url = "file://$W/pacote.tar.gz"
sha256 = "$SHA"
INDEOF
"$BIN" atualizar-indice "file://$W/indice-remoto.toml" >/dev/null || falha "atualizar-indice"
"$BIN" buscar primeiro | grep -q "ola-tuios" || falha "buscar no indice"
"$BIN" instalar ola-tuios >/dev/null || falha "instalar do indice"
"$BIN" listar --json | grep -q '"origem": "usuario"' || falha "instalar do indice nao instalou"
```

Obs.: heredoc `INDEOF` (nunca `EOF` — o shell externo interpretaria).

- [ ] **Step 2: Rodar o assert**

Run: `bash scripts-assert/apps.assert.sh`
Expected: `APPS-ASSERT-OK`

- [ ] **Step 3: Commit**

```bash
git add scripts-assert/apps.assert.sh
git commit -m "[TEST] — E2E file:// e indice no apps.assert (R21, R23-R26)"
```

---

### Task 5: Documentação — seção no guia `docs/tuios-apps.md`

**Files:**
- Modify: `docs/tuios-apps.md` (nova seção após "Ciclo de pacote")

- [ ] **Step 1: Inserir a seção**

Após a seção **Ciclo de pacote** (antes de **Exemplo de fábrica**):

````markdown
## Origens remotas e índice

```bash
# Download direto (http/https/file://) — timeout de 30 s
tuios-apps adicionar https://exemplo/ola-tuios-1.0.0.tar.gz

# Repositório git — git clone --depth 1 (120 s, sem prompt de credencial)
tuios-apps adicionar https://exemplo/meu-app.git

# Índice remoto (indice.toml) — grava em ${XDG_DATA_HOME:-~/.local/share}/tuios/
tuios-apps atualizar-indice https://exemplo/indice.toml
tuios-apps buscar termo [--json]
tuios-apps instalar <nome> [--sistema]   # baixa e confere o sha256 do índice
```

O índice é um `indice.toml` com entradas `[[app]]` (`nome`, `versao`,
`descricao`, `url`, `sha256` de 64 hex); gravação atômica e validação
completa — índice inválido **não** é gravado (exit 2). `instalar` confere o
sha256 do arquivo baixado contra o do índice antes de instalar (divergência =
exit 2, nada instalado).

**Rede só sob comando explícito:** `listar`/`info`/`rodar`/`validar`/
`empacotar`/`remover`/`menu` nunca acessam a rede. `git ausente` = exit 4.
````

- [ ] **Step 2: Conferir**

Run: `grep -n "Origens remotas" docs/tuios-apps.md`
Expected: 1 ocorrência; links internos continuam íntegros.

- [ ] **Step 3: Commit**

```bash
git add docs/tuios-apps.md
git commit -m "[DOC] — guia tuios-apps: origens remotas e indice (R21-R26)"
```

---

### Task 6: Regressão final (sandbox + ISO + test-all + test-install)

**Files:**
- Modify: apenas se um teste falhar (corrigir o bug apontado)

- [ ] **Step 1: checkPhase no sandbox já coberto nas Tasks 1/3** — refazer para
  garantir o estado final:

Run: `nix --store /tmp/nix-official build .#tuiosApps --no-link 2>&1 | tail -2`
Expected: sem erro (95 testes no sandbox, incl. git).

- [ ] **Step 2: ISO nova**

Run: `just iso 2>&1 | tail -2`
Expected: store path da ISO impressa, sem erro (eval + checkPhase).

- [ ] **Step 3: `just test-all`**

Run: `just test-all 2>&1 | tail -6`
Expected: `95 passed` + `APPS-ASSERT-OK` (com as 11 seções) +
`TODOS OS TESTES PASSARAM! ✅`

- [ ] **Step 4: `test-install` com a ISO nova (aceitação do Nível C)**

Run:
```bash
ISO="$(ls /tmp/nix-official$(readlink result)/iso/*.iso)" just test-install 2>&1 | tail -4
```
Expected: `INSTAL-TEST-OK (T1 instalação + T2 boot)`.

- [ ] **Step 5: Commit (apenas se houve correção)**

```bash
git add -A
git commit -m "[FIX] — correções apontadas pela regressão da Fase 3"
```
Sem correções → sem commit.

---

## Self-Review (do próprio plano)

**1. Cobertura da spec (R21–R26):** R21 → Task 1 (`baixar`/`materializar`) +
Tasks 3, 4 · R22 → Task 1 (`_clonar`, `GitAusente`) + Task 3 (exit 4) ·
R23 → Task 2 (`atualizar` atômico + validação) + Task 3 · R24 → Task 2
(`buscar`) + Task 3 (`--json`) + Task 4 · R25 → Task 2 (`baixar_e_conferir`)
+ Task 3 (`_cmd_instalar`) + Task 4 · R26 → constantes `TIMEOUT_*` em Task 1,
rede só nos comandos citados (Task 5 documenta), E2E sem rede (Task 4).
**Sem lacunas.**

**2. Placeholders:** nenhum TBD; todo passo tem código completo ou comando +
saída esperada.

**3. Consistência:** `materializar(origem, tmp)->Path` (1→3) · `baixar` pública
(1→2) · exceções `RedeFalhou/OrigemInvalida`→1, `ErroIndice`→2,
`IndiceNaoEncontrado`→1, `GitAusente`→4 (idênticas em Tasks 1-3) ·
`Entrada` frozen (2→3) · aliases `atualizar_indice/buscar_no_indice/
lookup_indice` evitam colisão com `discovery.buscar` (já importado) ·
`eh_remoto` decide antes de `materializar`; caminho local continua no ramo
antigo (compatibilidade) · XDG/TUIOS_* isolados por fixture em todos os
testes que leem estado · contagem 64+8+11+12 = **95** · heredocs internos
usam `INDEOF`/`FDEOF`, nunca `EOF` · git entra em `nativeCheckInputs`
(presença no sandbox = testes git rodam, não skipam).
