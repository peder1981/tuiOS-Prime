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
