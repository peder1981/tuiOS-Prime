import functools
import http.server
import shutil
import subprocess
import tarfile
import threading
from pathlib import Path

import pytest

from tuiosapps.origem import (
    GitAusente,
    OrigemInvalida,
    RedeFalhou,
    materializar,
)

TOML = (
    'nome = "ola-tuios"\n'
    'versao = "1.0.0"\n'
    'descricao = "Primeiro app tuiOS"\n'
    'entry = "ola.prw"\n'
)
PRW = 'User Function Ola()\nReturn .T.\n'
TEM_GIT = shutil.which("git") is not None


def test_materializar_file_url(tmp_path: Path):
    fonte = tmp_path / "fonte.bin"
    fonte.write_bytes(b"conteudo-remoto")
    saida = materializar(fonte.as_uri(), tmp_path / "work")
    assert saida == tmp_path / "work" / "pacote.tar.gz"
    assert saida.read_bytes() == b"conteudo-remoto"


def test_materializar_esquema_invalido(tmp_path: Path):
    with pytest.raises(OrigemInvalida):
        materializar("ftp://exemplo.invalido/x.tar.gz", tmp_path / "work")


def test_baixar_urlopen_falha(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    import urllib.error

    def boom(*args, **kwargs):
        raise urllib.error.URLError("sem rota")

    monkeypatch.setattr("urllib.request.urlopen", boom)
    with pytest.raises(RedeFalhou):
        materializar("https://exemplo.invalido/x.tar.gz", tmp_path / "work")


def test_baixar_download_vazio(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    class _Vazio:
        def __enter__(self):
            return self

        def __exit__(self, *args):
            return False

        def read(self, tamanho=None):
            return b""

    monkeypatch.setattr("urllib.request.urlopen", lambda *a, **k: _Vazio())
    with pytest.raises(RedeFalhou, match="vazio"):
        materializar("https://exemplo.invalido/x.tar.gz", tmp_path / "work")
    assert not (tmp_path / "work" / "pacote.tar.gz").exists()


def test_materializar_http_local(tmp_path: Path):
    fonte = tmp_path / "fonte.bin"
    fonte.write_bytes(b"http-conteudo")
    handler = functools.partial(
        http.server.SimpleHTTPRequestHandler, directory=str(tmp_path)
    )
    try:
        servidor = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    except OSError:
        pytest.skip("loopback indisponivel neste ambiente")
    thread = threading.Thread(target=servidor.serve_forever, daemon=True)
    thread.start()
    try:
        porta = servidor.server_address[1]
        saida = materializar(
            f"http://127.0.0.1:{porta}/fonte.bin", tmp_path / "work"
        )
        assert saida.read_bytes() == b"http-conteudo"
    finally:
        servidor.shutdown()
        servidor.server_close()


def _repo_git(base: Path) -> Path:
    repo = base / "meu-repo.git"
    repo.mkdir()
    ident = ["-c", "user.name=t", "-c", "user.email=t@t"]
    subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
    (repo / "tuios-app.toml").write_text(TOML, encoding="utf-8")
    (repo / "ola.prw").write_text(PRW, encoding="utf-8")
    subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
    subprocess.run(
        ["git", *ident, "commit", "-q", "-m", "inicio"], cwd=repo, check=True
    )
    return repo


@pytest.mark.skipif(not TEM_GIT, reason="git indisponivel")
def test_materializar_git_clone(tmp_path: Path):
    repo = _repo_git(tmp_path)
    saida = materializar(str(repo), tmp_path / "work")
    with tarfile.open(saida) as tar:
        nomes = tar.getnames()
    assert any(n.startswith("ola-tuios/") for n in nomes)
    assert not any(".git" in n.split("/") for n in nomes)


@pytest.mark.skipif(not TEM_GIT, reason="git indisponivel")
def test_materializar_git_clone_falha(tmp_path: Path):
    with pytest.raises(RedeFalhou, match="git clone falhou"):
        materializar(str(tmp_path / "nao-existe.git"), tmp_path / "work")


def test_materializar_git_ausente(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setattr("tuiosapps.origem.shutil.which", lambda *a: None)
    with pytest.raises(GitAusente):
        materializar("https://exemplo.invalido/app.git", tmp_path / "work")
