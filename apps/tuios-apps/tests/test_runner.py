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
printf '%s\\n' "$@" > "$OUT"
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
