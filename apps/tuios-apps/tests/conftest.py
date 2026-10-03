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
