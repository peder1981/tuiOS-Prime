"""Índice remoto de apps (R23-R25): atualizar / buscar / instalar com sha256."""
from __future__ import annotations

import hashlib
import os
import re
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
