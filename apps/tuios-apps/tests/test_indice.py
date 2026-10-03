import hashlib
from pathlib import Path

import pytest

from tuiosapps.indice import (
    ErroIndice,
    IndiceNaoEncontrado,
    atualizar,
    baixar_e_conferir,
    buscar,
    caminho_indice,
    carregar,
    lookup,
)

SHA_OK = "a" * 64
INDICE_OK = (
    "[[app]]\n"
    'nome = "ola-tuios"\n'
    'versao = "1.0.0"\n'
    'descricao = "Primeiro app tuiOS"\n'
    'url = "https://exemplo.invalido/ola-tuios-1.0.0.tar.gz"\n'
    f'sha256 = "{SHA_OK}"\n'
    "\n"
    "[[app]]\n"
    'nome = "outro-app"\n'
    'versao = "2.0.0"\n'
    'descricao = "Segundo app utilitario"\n'
    'url = "https://exemplo.invalido/outro-app-2.0.0.tar.gz"\n'
    f'sha256 = "{SHA_OK}"\n'
)


@pytest.fixture
def xdg(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    base = tmp_path / "xdg"
    monkeypatch.setenv("XDG_DATA_HOME", str(base))
    return base


def _publicar(tmp_path: Path, conteudo: str) -> str:
    fonte = tmp_path / "indice-remoto.toml"
    fonte.write_text(conteudo, encoding="utf-8")
    return fonte.as_uri()


def test_atualizar_indice_ok(xdg: Path, tmp_path: Path):
    atualizar(_publicar(tmp_path, INDICE_OK))
    assert caminho_indice().is_file()
    entradas = carregar()
    assert [e.nome for e in entradas] == ["ola-tuios", "outro-app"]
    assert entradas[0].sha256 == SHA_OK


def test_atualizar_sha_invalido_nao_grava(xdg: Path, tmp_path: Path):
    ruim = INDICE_OK.replace(SHA_OK, "b" * 63, 1)
    with pytest.raises(ErroIndice, match="sha256"):
        atualizar(_publicar(tmp_path, ruim))
    assert not caminho_indice().exists()


def test_atualizar_url_sem_esquema(xdg: Path, tmp_path: Path):
    ruim = INDICE_OK.replace("https://", "ftp://", 1)
    with pytest.raises(ErroIndice, match="esquema"):
        atualizar(_publicar(tmp_path, ruim))
    assert not caminho_indice().exists()


def test_atualizar_campos_faltando(xdg: Path, tmp_path: Path):
    ruim = INDICE_OK.replace('descricao = "Primeiro app tuiOS"\n', "", 1)
    with pytest.raises(ErroIndice, match="sem"):
        atualizar(_publicar(tmp_path, ruim))


def test_atualizar_app_duplicado(xdg: Path, tmp_path: Path):
    ruim = (
        INDICE_OK
        + '\n[[app]]\nnome = "ola-tuios"\nversao = "9.9.9"\n'
        + 'descricao = "x"\nurl = "https://e.invalido/x.tar.gz"\n'
        + f'sha256 = "{SHA_OK}"\n'
    )
    with pytest.raises(ErroIndice, match="duplicado"):
        atualizar(_publicar(tmp_path, ruim))


def test_carregar_sem_indice(xdg: Path):
    with pytest.raises(IndiceNaoEncontrado, match="atualizar-indice"):
        carregar()


def test_buscar_filtro(xdg: Path, tmp_path: Path):
    atualizar(_publicar(tmp_path, INDICE_OK))
    assert [e.nome for e in buscar("primeiro")] == ["ola-tuios"]
    assert [e.nome for e in buscar("UTILITARIO")] == ["outro-app"]


def test_buscar_sem_correspondencia(xdg: Path, tmp_path: Path):
    atualizar(_publicar(tmp_path, INDICE_OK))
    assert buscar("inexistente") == []


def test_lookup_nao_achado(xdg: Path, tmp_path: Path):
    atualizar(_publicar(tmp_path, INDICE_OK))
    assert lookup("fantasma") is None
    assert lookup("ola-tuios").versao == "1.0.0"


def test_baixar_e_conferir_ok(xdg: Path, tmp_path: Path):
    tarball = tmp_path / "app.tar.gz"
    tarball.write_bytes(b"pacote-legal")
    sha = hashlib.sha256(b"pacote-legal").hexdigest()
    conteudo = INDICE_OK.replace(
        "https://exemplo.invalido/ola-tuios-1.0.0.tar.gz", tarball.as_uri(), 1
    ).replace(SHA_OK, sha, 1)
    atualizar(_publicar(tmp_path, conteudo))
    entrada = lookup("ola-tuios")
    saida = baixar_e_conferir(entrada, tmp_path / "work")
    assert saida.read_bytes() == b"pacote-legal"


def test_baixar_e_conferir_sha_divergente(xdg: Path, tmp_path: Path):
    tarball = tmp_path / "app.tar.gz"
    tarball.write_bytes(b"pacote-adulterado")
    conteudo = INDICE_OK.replace(
        "https://exemplo.invalido/ola-tuios-1.0.0.tar.gz", tarball.as_uri(), 1
    )
    atualizar(_publicar(tmp_path, conteudo))  # sha no índice é SHA_OK (a*64)
    with pytest.raises(ErroIndice, match="sha256 divergente"):
        baixar_e_conferir(lookup("ola-tuios"), tmp_path / "work")
    assert not (tmp_path / "work" / "pacote.tar.gz").exists()
