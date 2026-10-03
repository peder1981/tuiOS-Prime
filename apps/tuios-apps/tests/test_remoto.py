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
    assert (
        main(["atualizar-indice", _indice_local(tmp_path, tarball, sha="b" * 64)])
        == 0
    )
    assert main(["instalar", "ola-tuios"]) == 2
    assert not (usuario / "ola-tuios").exists()
