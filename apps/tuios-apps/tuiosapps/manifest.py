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
