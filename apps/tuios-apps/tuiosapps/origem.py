"""Origens remotas: download http(s)/file e clone git (R21-R22, R26)."""
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
