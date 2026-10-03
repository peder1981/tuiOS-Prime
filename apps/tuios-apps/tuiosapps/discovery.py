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
