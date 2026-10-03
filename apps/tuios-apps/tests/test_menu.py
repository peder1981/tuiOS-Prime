from pathlib import Path
from types import SimpleNamespace
import sys

import pytest

from tuiosapps.cli import main

FAKE = f'''#!{sys.executable}
import os, sys, pathlib
log = pathlib.Path(os.environ["FAKE_LOG"])
with log.open("a", encoding="utf-8") as f:
    f.write(" ".join(sys.argv[1:]).replace("\\n", " ") + "\\n")
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
    import os
    monkeypatch.setenv("PATH", str(bin_dir) + os.pathsep + os.environ["PATH"])

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
