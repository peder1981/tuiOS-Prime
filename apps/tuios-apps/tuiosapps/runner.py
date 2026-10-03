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
